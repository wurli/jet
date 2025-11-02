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
    // Use the system-installed python3 kernel
    let kernel_path = std::env::var("IPYKERNEL_PATH")
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").expect("HOME not set");
            format!("{}/Library/Jupyter/kernels/python3/kernel.json", home)
        });
    
    jet::api::start_kernel(kernel_path.into())
        .expect("Failed to start ipykernel")
        .0
}

fn execute(code: &str) -> impl Fn() -> Option<Message> {
    api::execute_code(ipykernel_id(), String::from(code), HashMap::new()).expect("Could not execute code")
}

#[test]
fn test_ipykernel_can_run_simple_code() {
    let callback = execute("1 + 1");

    let res = callback().expect("Callback returned `None`");

    // Initial callback should give the execute result
    assert_matches!(res, Message::ExecuteResult(msg) => {
        assert_eq!(msg.content.data["text/plain"], "2")
    });

    // The following callback should give None
    assert_matches!(callback(), None);
}

#[test]
fn test_ipykernel_persists_environment() {
    let callback = execute("x = 1");

    // The callback shouldn't have an output
    assert_matches!(callback(), None);

    let callback = execute("x");

    let res = callback().expect("Callback returned `None`");

    // Initial callback should give the execute result
    assert_matches!(res, Message::ExecuteResult(msg) => {
        assert_eq!(msg.content.data["text/plain"], "1")
    });
}

#[test]
fn test_ipykernel_returns_stdout() {
    let callback = execute("print('Hi!', end='')");

    let res = callback().expect("Callback returned `None`");

    assert_matches!(res, Message::Stream(msg) => {
        assert_eq!(msg.content.text, "Hi!")
    });

    // The following callback should give None
    assert_matches!(callback(), None);
}

#[test]
fn test_ipykernel_handles_stdin() {
    let callback = execute("input('Enter something:')");

    let res = callback().expect("Callback returned `None`");

    assert_matches!(res, Message::InputRequest(msg) => {
        assert_eq!(msg.content.prompt, "Enter something:")
    });

    api::provide_stdin(&ipykernel_id(), String::from("Hello tests!")).expect("Could not provide stdin");

    let res = callback().expect("Callback returned `None`");

    assert_matches!(res, Message::ExecuteResult(msg) => {
        assert_matches!(msg.content.data["text/plain"], Value::String(ref string) => {
            assert_eq!(string, "'Hello tests!'")
        })
    });

    // The following callback should give None
    assert_matches!(callback(), None);
}

#[test]
fn test_ipykernel_streams_results() {
    // Print "a" then "b" at 0.5s intervals
    // Use sys.stdout.flush() to ensure output is sent immediately
    let callback = execute("import time\nimport sys\nprint('a', end='', flush=True)\ntime.sleep(0.5)\nprint('b', end='', flush=True)");

    // Receive the first result
    let res = callback().expect("Callback returned `None`");

    // We only set the timer after we receive the first result. This is because tests may be
    // run in parallel, meaning the kernel may be busy executing other stuff when we first send the
    // execute request. Once we get the first 'a' through, we should expect the 'b' to come through
    // within 0.5s.
    let execute_time = Instant::now();

    assert_matches!(res, Message::Stream(msg) => {
        assert_eq!(msg.content.text, "a")
    });

    // Receive the second result
    let res = callback().expect("Callback returned `None`");
    let elapsed = execute_time.elapsed();

    assert_matches!(res, Message::Stream(msg) => {
        assert_eq!(msg.content.text, "b")
    });

    assert!(
        Duration::from_millis(400) < elapsed,
        "Result received too early: {}ms after request)",
        elapsed.as_millis()
    );
    assert!(
        elapsed < Duration::from_millis(700),
        "Result received too late: {}ms after request",
        elapsed.as_millis()
    );

    // The following callback should give None
    assert_matches!(callback(), None);
}

#[test]
fn test_ipykernel_is_complete_request() {
    // This test runs on a new instance of ipykernel since is_complete requests seem to block the
    // kernel from returning stdout, which can interfere with other tests (but only if
    // running with multiple threads)
    let id = start_ipykernel();
    let is_complete = |code: &str| -> Message {
        api::is_complete(id.clone(), String::from(code))
            .expect("Could not execute is_complete request")
    };

    assert_matches!(is_complete("1"), Message::IsCompleteReply(msg) => {
        assert_matches!(msg.content.status, IsComplete::Complete)
    });

    assert_matches!(is_complete("for i in range(3):"), Message::IsCompleteReply(msg) => {
        assert_matches!(msg.content.status, IsComplete::Incomplete)
    });

    assert_matches!(is_complete("$"), Message::IsCompleteReply(msg) => {
        assert_matches!(msg.content.status, IsComplete::Invalid)
    });

    api::request_shutdown(&id).expect("Could not shut down ipykernel");
}
