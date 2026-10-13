use lamprey_macros::record;

use crate::v1::types::{ApplicationId, ChannelId, MessageId};

/// create a new interaction
#[record]
pub struct Create {
    pub application_id: ApplicationId,

    #[serde(flatten)]
    pub ty: CreateType,
}

#[record]
#[serde(tag = "type")]
pub enum CreateType {
    Button(Button),
}

/// a button was pressed
#[record]
pub struct Button {
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub custom_id: String,
}

impl CreateType {
    pub fn channel_id(&self) -> Option<ChannelId> {
        match self {
            Self::Button(a) => Some(a.channel_id),
        }
    }
}
