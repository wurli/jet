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
use jet::msg::wire::is_complete_reply::IsComplete;
use jet::msg::wire::jupyter_message::Message;
use jet::msg::wire::message_id::Id;

static ARK_ID: OnceLock<Id> = OnceLock::new();

/// Get the Id for the Ark kernel. 'Getting' for the first time starts the kernel so we can use
/// the same session for all tests, even though they're run in parallel.
fn ark_id() -> Id {
    ARK_ID
        .get_or_init(|| {
            let (id, _info) = jet::api::start_kernel("kernels/ark/kernel.json".into())
                .expect("Failed to start Ark");
            id
        })
        .clone()
}

fn execute(code: &str) -> impl Fn() -> Option<Message> {
    api::execute_code(ark_id(), String::from(code), HashMap::new()).expect("Could not execute code")
}

#[test]
fn test_ark_can_run_simple_code() {
    let callback = execute("1 + 1");

    let res = callback().expect("Callback returned `None`");

    // Initial callback should give the execute result
    assert_matches!(res, Message::ExecuteResult(msg) => {
        assert_eq!(msg.content.data["text/plain"], "[1] 2")
    });

    // The following callback should give None
    assert_matches!(callback(), None);
}

#[test]
fn test_ark_persists_environment() {
    let callback = execute("x <- 1");

    // The callback shouldn't have an output
    assert_matches!(callback(), None);

    let callback = execute("x");

    let res = callback().expect("Callback returned `None`");

    // Initial callback should give the execute result
    assert_matches!(res, Message::ExecuteResult(msg) => {
        assert_eq!(msg.content.data["text/plain"], "[1] 1")
    });
}

#[test]
fn test_ark_returns_stdout() {
    let callback = execute("cat('Hi!')");

    let res = callback().expect("Callback returned `None`");

    assert_matches!(res, Message::Stream(msg) => {
        assert_eq!(msg.content.text, "Hi!")
    });

    // The following callback should give None
    assert_matches!(callback(), None);
}

#[test]
fn test_ark_streams_results() {
    // It's important we only print a single character here, since Ark gathers stdout from R
    // at regular intervals, meaning something like `cat("hello")` might only return output
    // in several chunks.
    //
    // Here we just print "a" then "b" at 0.5s intervals
    let callback = execute("for (letter in c('a', 'b')) { Sys.sleep(0.5); cat(letter) }");
    let execute_time = Instant::now();

    // Receive the first result
    let res = callback().expect("Callback returned `None`");
    let elapsed = execute_time.elapsed();

    assert_matches!(res, Message::Stream(msg) => {
        assert_eq!(msg.content.text, "a")
    });

    assert!(
        Duration::from_millis(500) < elapsed,
        "Result received too early: {}ms after request)",
        elapsed.as_millis()
    );
    assert!(
        elapsed < Duration::from_millis(700),
        "Result received too late: {}ms after request",
        elapsed.as_millis()
    );

    // Receive the second result
    let res = callback().expect("Callback returned `None`");
    let elapsed = execute_time.elapsed();

    assert_matches!(res, Message::Stream(msg) => {
        assert_eq!(msg.content.text, "b")
    });

    assert!(
        Duration::from_millis(1000) < elapsed,
        "Result received too early: {}ms after request)",
        elapsed.as_millis()
    );
    assert!(
        elapsed < Duration::from_millis(1200),
        "Result received too late: {}ms after request",
        elapsed.as_millis()
    );

    // The following callback should give None
    assert_matches!(callback(), None);
}

fn is_complete(code: &str) -> Message {
    api::is_complete(ark_id(), String::from(code)).expect("Could not execute is_complete request")
}

#[test]
fn test_ark_is_complete_request() {
    assert_matches!(is_complete("1"), Message::IsCompleteReply(msg) => {
        assert_matches!(msg.content.status, IsComplete::Complete)
    });

    assert_matches!(is_complete("1 +"), Message::IsCompleteReply(msg) => {
        assert_matches!(msg.content.status, IsComplete::Incomplete)
    });

    assert_matches!(is_complete("_"), Message::IsCompleteReply(msg) => {
        assert_matches!(msg.content.status, IsComplete::Invalid)
    });
}
