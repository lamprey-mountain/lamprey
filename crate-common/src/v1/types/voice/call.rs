use lamprey_macros::record;

#[cfg(feature = "utoipa")]
use utoipa::IntoParams;

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;
use crate::v1::types::{ChannelId, RoomId, misc::Time};

/// a currently active voice session
#[record]
pub struct CallMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,

    /// when this call was created
    ///
    /// roughly corresponds to the time that the first user joined
    pub created_at: Time,

    /// how many people are in the audience
    ///
    /// only populated if this is a broadcast channel. in broadcast channels,
    /// only voice states for yourself and speakers (ie. users who are not
    /// suppressed) are sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience_count: Option<u64>,
}

/// a currently active voice session, with ids
#[record]
pub struct Call {
    pub channel_id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    #[serde(flatten)]
    pub inner: CallMetadata,
}

#[record]
pub struct CallCreate {
    /// call topic
    ///
    /// must have VoiceMute permission in target channel to set
    // NOTE: unsure about using this permission
    #[schema(min_length = 1, max_length = 512)]
    #[validate(length(min = 1, max = 512))]
    pub topic: Option<String>,
}

#[record]
pub struct CallPatch {
    /// the current call topic
    ///
    /// only unsuppressed users can change the call topic
    #[schema(min_length = 1, max_length = 512)]
    #[validate(length(min = 1, max = 512))]
    #[serde(default, deserialize_with = "some_option")]
    pub topic: Option<Option<String>>,
}

#[record]
#[cfg_attr(feature = "utoipa", derive(IntoParams))]
pub struct CallDeleteParams {
    /// if people are still connected to this channel, try to forcibly disconnect them
    ///
    /// requires VoiceDisconnect permission
    #[serde(default)]
    pub force: bool,
}

impl Call {
    pub fn apply_patch(&mut self, patch: CallPatch) {
        if let Some(topic) = patch.topic {
            self.inner.topic = topic;
        }
    }
}
