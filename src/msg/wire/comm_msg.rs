/*
 * comm_msg.rs
 *
 * Copyright (C) 2022 Posit Software, PBC. All rights reserved.
 *
 */

use serde::Deserialize;
use serde::Serialize;

use crate::msg::wire::jupyter_message::Describe;

/// Represents a message on a custom comm channel.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommWireMsg {
    pub comm_id: String,
    pub data: serde_json::Value,
}

impl Describe for CommWireMsg {
    fn message_type() -> String {
        String::from("comm_msg")
    }
}
