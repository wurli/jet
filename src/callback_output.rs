use crate::msg::wire::jupyter_message::Message;

pub enum CallbackOutput {
    Busy(Option<Message>),
    Idle,
}
