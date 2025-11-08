/*
 * ipykernel.rs
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

static IPYKERNEL_ID: OnceLock<Id> = OnceLock::new();

/// Get the Id for the ipykernel. 'Getting' for the first time starts the kernel so we can use
/// the same session for all tests, even though they're run in parallel.
fn ipykernel_id() -> Id {
    IPYKERNEL_ID.get_or_init(start_ipykernel).clone()
}

fn start_ipykernel() -> Id {
    let kernels = api::list_available_kernels();

    let ipykernel_path = kernels
        .iter()
        .filter_map(|(path, spec)| {
            if spec.display_name == String::from("Python 3 (ipykernel)") {
                Some(path)
            } else {
                None
            }
        })
        .next()
        .expect("Ipykernel could not be located");

    jet::api::start_kernel(ipykernel_path.to_owned())
        .expect("Failed to start ipykernel")
        .0
}

fn execute(code: &str) -> impl Fn() -> CallbackOutput {
    execute_in(ipykernel_id(), code)
}

fn execute_in(id: Id, code: &str) -> impl Fn() -> CallbackOutput {
    let callback =
        api::execute_code(id, String::from(code), HashMap::new()).expect("Could not execute code");
    // We should always get an ExecuteInput message first
    assert_matches!(await_result(&callback), Some(Message::ExecuteInput(msg)) => {
        assert_eq!(msg.content.code, code)
    });
    callback
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
fn test_ipykernel_can_run_simple_code() {
    let callback = execute("1 + 1");

    let res = await_result(&callback).expect("Callback returned `None`");

    // Initial callback should give the execute result
    assert_matches!(res, Message::ExecuteResult(msg) => {
        assert_eq!(msg.content.data["text/plain"], "2")
    });

    // The following callback should give None
    assert_matches!(await_result(&callback), None);
}

#[test]
fn test_ipykernel_persists_environment() {
    let callback = execute("x = 1");

    // The callback shouldn't have an output
    assert_matches!(await_result(&callback), None);

    let callback = execute("x");

    let res = await_result(&callback).expect("Callback returned `None`");

    // Initial callback should give the execute result
    assert_matches!(res, Message::ExecuteResult(msg) => {
        assert_eq!(msg.content.data["text/plain"], "1")
    });
}

#[test]
fn test_ipykernel_returns_stdout() {
    let callback = execute("print('Hi!', end='')");

    let res = await_result(&callback).expect("Callback returned `None`");

    assert_matches!(res, Message::Stream(msg) => {
        assert_eq!(msg.content.text, "Hi!")
    });

    // The following callback should give None
    assert_matches!(await_result(&callback), None);
}

#[test]
fn test_ipykernel_handles_stdin() {
    let callback = execute("input('Enter something:')");

    let res = await_result(&callback).expect("Callback returned `None`");

    assert_matches!(res, Message::InputRequest(msg) => {
        assert_eq!(msg.content.prompt, "Enter something:")
    });

    api::provide_stdin(&ipykernel_id(), String::from("Hello tests!"))
        .expect("Could not provide stdin");

    let res = await_result(&callback).expect("Callback returned `None`");

    assert_matches!(res, Message::ExecuteResult(msg) => {
        assert_matches!(msg.content.data["text/plain"], Value::String(ref string) => {
            assert_eq!(string, "'Hello tests!'")
        })
    });

    // The following callback should give None
    assert_matches!(await_result(&callback), None);
}

#[test]
fn test_ipykernel_streams_results() {
    // Print "a" then "b" at 0.5s intervals
    // Use sys.stdout.flush() to ensure output is sent immediately
    let callback = execute_in(
        start_ipykernel(),
        "import time\nimport sys\nprint('a', end='', flush=True)\ntime.sleep(0.5)\nprint('b', end='', flush=True)",
    );

    // Receive the first result
    let res = await_result(&callback).expect("Callback returned `None`");

    // We only set the timer after we receive the first result. This is because tests may be
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
        elapsed < Duration::from_millis(700),
        "Second result received too late: {}ms after first",
        elapsed.as_millis()
    );

    // The following callback should give None
    assert_matches!(await_result(&callback), None);
}

fn is_complete(code: &str) -> impl Fn() -> CallbackOutput {
    api::is_complete(ipykernel_id(), String::from(code))
        .expect("Could not send is_complete request")
}

#[test]
fn test_ipykernel_provides_code_completeness() {
    assert_matches!(await_result(&is_complete("1")), Some(Message::IsCompleteReply(msg)) => {
        assert_matches!(msg.content.status, IsComplete::Complete)
    });

    assert_matches!(await_result(&is_complete("for i in range(3):")), Some(Message::IsCompleteReply(msg)) => {
        assert_matches!(msg.content.status, IsComplete::Incomplete)
    });

    assert_matches!(await_result(&is_complete("$")), Some(Message::IsCompleteReply(msg)) => {
        assert_matches!(msg.content.status, IsComplete::Invalid)
    });
}

fn get_completions(code: &str, pos: u32) -> impl Fn() -> CallbackOutput {
    api::get_completions(ipykernel_id(), String::from(code), pos)
        .expect("Could not execute is_complete request")
}

#[test]
fn test_ipykernel_provides_completions() {
    let code = "my_long_named_variable = 1\nmy_long_";
    let callback = get_completions(code, code.chars().count() as u32);
    assert_matches!(await_result(&callback), Some(Message::CompleteReply(msg)) => {
        assert_eq!(
            msg.content.matches.into_iter().next().expect("No completions returned"),
            String::from("my_long_named_variable")
        )
    });
}
