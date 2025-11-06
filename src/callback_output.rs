use crate::msg::wire::jupyter_message::Message;

#[derive(Debug)]
pub enum CallbackOutput {
    Busy(Option<Message>),
    Idle,
}
