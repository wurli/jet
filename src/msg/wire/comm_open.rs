/*
 * comm_open.rs
 *
 * Copyright (C) 2022 Posit Software, PBC. All rights reserved.
 *
 */

use serde::Deserialize;
use serde::Serialize;

use crate::msg::wire::jupyter_message::Describe;
use crate::msg::wire::message_id::Id;

/// Represents a request to open a custom comm
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommOpen {
    pub comm_id: Id,
    pub target_name: String,
    pub data: serde_json::Value,
}

impl Describe for CommOpen {
    fn message_type() -> String {
        String::from("comm_open")
    }
}
