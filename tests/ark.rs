/*
 * ark.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use assert_matches::assert_matches;
use jet::api;
use jet::callback_output::CallbackOutput;
use jet::msg::wire::is_complete_reply::IsComplete;
use jet::msg::wire::jupyter_message::Message;
use jet::msg::wire::message_id::Id;
use serde_json::Value;

static ARK_ID: OnceLock<Id> = OnceLock::new();

/// Get the Id for the Ark kernel. 'Getting' for the first time starts the kernel so we can use
/// the same session for all tests, even though they're run in parallel.
fn ark_id() -> Id {
    ARK_ID.get_or_init(start_ark).clone()
}

fn start_ark() -> Id {
    let kernels = api::list_available_kernels();

    let ark_path = kernels
        .iter()
        .filter_map(|(path, spec)| {
            if spec.display_name == String::from("Ark R Kernel") {
                Some(path)
            } else {
                None
            }
        })
        .next()
        .expect("Ark kernel could not be located");

    jet::api::start_kernel(ark_path.to_owned())
        .expect("Failed to start Ark")
        .0
}

fn execute(code: &str) -> impl Fn() -> CallbackOutput {
    execute_in(ark_id(), code)
}

fn execute_in(id: Id, code: &str) -> impl Fn() -> CallbackOutput {
    api::execute_code(id, String::from(code), HashMap::new()).expect("Could not execute code")
}

fn await_result(callback: &impl Fn() -> CallbackOutput) -> Option<Message> {
    loop {
        match callback() {
            CallbackOutput::Idle => return None,
            CallbackOutput::Busy(Some(msg)) => return Some(msg),
            CallbackOutput::Busy(None) => {}
        }
    }
}

#[test]
fn test_ark_can_run_simple_code() {
    let callback = execute("1 + 1");

    let res = await_result(&callback).expect("Callback returned `None`");

    // Initial callback should give the execute result
    assert_matches!(res, Message::ExecuteResult(msg) => {
        assert_eq!(msg.content.data["text/plain"], "[1] 2")
    });

    // The following callback should give None
    assert_matches!(await_result(&callback), None);
}

#[test]
fn test_ark_persists_environment() {
    let callback = execute("x <- 1");

    // The callback shouldn't have an output
    assert_matches!(await_result(&callback), None);

    let callback = execute("x");

    let res = await_result(&callback).expect("Callback returned `None`");

    // Initial callback should give the execute result
    assert_matches!(res, Message::ExecuteResult(msg) => {
        assert_eq!(msg.content.data["text/plain"], "[1] 1")
    });
}

#[test]
fn test_ark_returns_stdout() {
    let callback = execute("cat('Hi!')");

    let res = await_result(&callback).expect("Callback returned `None`");

    assert_matches!(res, Message::Stream(msg) => {
        assert_eq!(msg.content.text, "Hi!")
    });

    // The following callback should give None
    assert_matches!(await_result(&callback), None);
}

#[test]
fn test_ark_handles_stdin() {
    let callback = execute("readline('Enter something:')");

    let res = await_result(&callback).expect("Callback returned `None`");

    assert_matches!(res, Message::InputRequest(msg) => {
        assert_eq!(msg.content.prompt, "Enter something:")
    });

    api::provide_stdin(&ark_id(), String::from("Hello tests!")).expect("Could not provide stdin");

    let res = await_result(&callback).expect("Callback returned `None`");

    assert_matches!(res, Message::ExecuteResult(msg) => {
        assert_matches!(msg.content.data["text/plain"], Value::String(ref string) => {
            assert_eq!(string, "[1] \"Hello tests!\"")
        })
    });

    // The following callback should give None
    assert_matches!(await_result(&callback), None);
}

#[test]
fn test_ark_streams_results() {
    // It's important we only print a single character here, since Ark gathers stdout from R
    // at regular intervals, meaning something like `cat("hello")` might only return output
    // in several chunks.
    //
    // Here we just print "a" then "b" at 0.5s intervals
    let callback = execute_in(start_ark(), "cat('a')\nSys.sleep(0.5)\ncat('b')");

    // Receive the first result
    let res = await_result(&callback).expect("Callback returned `None`");

    // We only set the timer afer we receive the first result. This is because tests may be
    // run in parallel, meaning the kernel may be busy executing other stuff when we first send the
    // execute request. Once we get the first 'a' through, we should expect the 'b' to come through
    // within 0.5s.
    let execute_time = Instant::now();

    assert_matches!(res, Message::Stream(msg) => {
        assert_eq!(msg.content.text, "a")
    });

    // Receive the second result
    let res = await_result(&callback).expect("Callback returned `None`");
    let elapsed = execute_time.elapsed();

    assert_matches!(res, Message::Stream(msg) => {
        assert_eq!(msg.content.text, "b")
    });

    assert!(
        Duration::from_millis(400) < elapsed,
        "Second result received too early: {}ms after first)",
        elapsed.as_millis()
    );
    assert!(
        elapsed < Duration::from_millis(600),
        "Second result received too late: {}ms after first",
        elapsed.as_millis()
    );

    // The following callback should give None
    assert_matches!(await_result(&callback), None);
}

fn is_complete(code: &str) -> impl Fn() -> CallbackOutput {
    api::is_complete(ark_id(), String::from(code)).expect("Could not send is_complete request")
}

#[test]
fn test_ark_provides_code_completeness() {
    let callback = is_complete("1 + 1");
    assert_matches!(await_result(&callback), Some(Message::IsCompleteReply(msg)) => {
        assert_matches!(msg.content.status, IsComplete::Complete)
    });

    let callback = is_complete("1 +");
    assert_matches!(await_result(&callback), Some(Message::IsCompleteReply(msg)) => {
        assert_matches!(msg.content.status, IsComplete::Incomplete)
    });

    let callback = is_complete("_");
    assert_matches!(await_result(&callback), Some(Message::IsCompleteReply(msg)) => {
        assert_matches!(msg.content.status, IsComplete::Invalid)
    });
}
