/*
 * execute_reply.rs
 *
 * Copyright (C) 2022 Posit Software, PBC. All rights reserved.
 *
 */

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::msg::wire::jupyter_message::Describe;
use crate::msg::wire::jupyter_message::Status;

/// Represents a reply from an execute_request message
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecuteReply {
    /// The status of the request
    pub status: Status,

    /// Monotonically increasing execution counter
    pub execution_count: u32,

    /// Results for user expressions
    pub user_expressions: Value,
}

impl Describe for ExecuteReply {
    fn message_type() -> String {
        String::from("execute_reply")
    }
}
