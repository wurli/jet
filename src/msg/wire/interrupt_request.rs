/*
 * interrupt_request.rs
 *
 * Copyright (C) 2022 Posit Software, PBC. All rights reserved.
 *
 */

use serde::Deserialize;
use serde::Serialize;

use crate::msg::wire::jupyter_message::Describe;

/// Represents request from the frontend to the kernel to interrupt execution
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterruptRequest {}

impl Describe for InterruptRequest {
    fn message_type() -> String {
        String::from("interrupt_request")
    }
}
