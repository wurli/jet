/*
 * comm_close.rs
 *
 * Copyright (C) 2022 Posit Software, PBC. All rights reserved.
 *
 */

use serde::Deserialize;
use serde::Serialize;

use crate::msg::wire::jupyter_message::Describe;
use crate::msg::wire::message_id::Id;

/// Represents a request to close a Jupyter communication channel that was
/// previously opened with a comm_open message.
///
/// (https://jupyter-client.readthedocs.io/en/stable/messaging.html#comm-close)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommClose {
    pub comm_id: Id,
}

impl Describe for CommClose {
    fn message_type() -> String {
        String::from("comm_close")
    }
}
