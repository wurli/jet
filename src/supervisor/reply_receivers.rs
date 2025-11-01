/*
 * reply_receivers.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use std::sync::mpsc::Receiver;

use crate::msg::wire::{jupyter_message::Message, message_id::Id};

/// When you send a request on stdin, any replies which come back from the kernel will be routed
/// via these sockets. This allows you to handle replies _only_ related to the original request,
/// without worrying about dropping any unrelated messages.
pub struct ReplyReceivers {
    /// The ID of the original request message
    pub id: Id,
    /// A receiver for replies to `id` on the iopub socket
    pub iopub: Receiver<Message>,
    /// A receiver for replies to `id` on the shell socket
    pub shell: Receiver<Message>,
    /// A receiver for replies to `id` on the stdin socket
    pub stdin: Receiver<Message>,
    /// A receiver for replies to `id` on the control socket
    pub control: Receiver<Message>,
}
