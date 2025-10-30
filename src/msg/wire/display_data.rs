/*
 * display_data.rs
 *
 * Copyright (C) 2023 Posit Software, PBC. All rights reserved.
 *
 */

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::msg::wire::jupyter_message::Describe;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayData {
    /// The data giving the MIME key/value pairs to display
    pub data: HashMap<String, Value>,

    /// Optional additional metadata
    pub metadata: HashMap<String, Value>,

    /// Optional transient data
    pub transient: HashMap<String, Value>,
}

impl Describe for DisplayData {
    fn message_type() -> String {
        String::from("display_data")
    }
}
