/*
 * is_complete_request.rs
 *
 * Copyright (C) 2022 Posit Software, PBC. All rights reserved.
 *
 */

use serde::Deserialize;
use serde::Serialize;

use crate::msg::wire::jupyter_message::Describe;

/// Represents a request from the frontend to test a code fragment to for
/// completeness.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IsCompleteRequest {
    pub code: String,
}

impl Describe for IsCompleteRequest {
    fn message_type() -> String {
        String::from("is_complete_request")
    }
}
