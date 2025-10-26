use crate::v1::types::{util::some_option, ChannelId, MediaId, RoomId, WebhookId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Webhook {
    pub id: WebhookId,
    pub room_id: Option<RoomId>,
    // TODO: rename to channel_id
    pub thread_id: ChannelId,
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,
    pub avatar: Option<MediaId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct WebhookCreate {
    #[validate(length(min = 1, max = 64))]
    pub name: String,
    pub avatar: Option<MediaId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct WebhookUpdate {
    pub thread_id: Option<ChannelId>,
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "some_option")]
    pub avatar: Option<Option<MediaId>>,
    #[serde(default)]
    pub rotate_token: bool,
}
