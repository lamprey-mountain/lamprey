//! random utilities to make writing the bot easier

use common::v1::types::MessageCreate;

/// what the bot should respond with
pub struct BotResponse(pub Option<MessageCreate>);

impl From<&str> for BotResponse {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl From<String> for BotResponse {
    fn from(value: String) -> Self {
        Self(Some(MessageCreate {
            content: Some(value),
            ..Default::default()
        }))
    }
}

impl From<MessageCreate> for BotResponse {
    fn from(value: MessageCreate) -> Self {
        Self(Some(value))
    }
}

impl From<()> for BotResponse {
    fn from(_: ()) -> Self {
        Self(None)
    }
}

impl<T: Into<BotResponse>> From<Option<T>> for BotResponse {
    fn from(value: Option<T>) -> Self {
        Self(value.and_then(|value| value.into().0))
    }
}
