/*
 * execute_result.rs
 *
 * Copyright (C) 2022 Posit Software, PBC. All rights reserved.
 *
 */

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::msg::wire::jupyter_message::Describe;

/// Represents a request from the frontend to execute code
///
/// Docs: https://jupyter-client.readthedocs.io/en/latest/messaging.html#id7
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecuteResult {
    /// The data giving the result of the execution
    /// Should be the same as `DisplayData.data`
    pub data: HashMap<String, Value>,

    /// A monotonically increasing execution counter
    pub execution_count: u32,

    /// Optional additional metadata
    /// Should be the same as `DisplayData.metadata`
    pub metadata: HashMap<String, Value>,
}

impl Describe for ExecuteResult {
    fn message_type() -> String {
        String::from("execute_result")
    }
}
