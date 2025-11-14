use crate::msg::wire::jupyter_message::Message;

#[derive(Debug)]
pub enum KernelResponse {
    Busy(Option<Message>),
    Idle,
}
