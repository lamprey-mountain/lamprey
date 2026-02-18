use crate::v1::types::{ChannelId, MediaId, RoomId, UserId, WebhookId};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;
#[cfg(feature = "validator")]
use validator::Validate;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Webhook {
    pub id: WebhookId,
    pub room_id: Option<RoomId>,
    pub channel_id: ChannelId,
    pub creator_id: Option<UserId>,
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,
    pub avatar: Option<MediaId>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub token: Option<String>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct WebhookCreate {
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,
    pub avatar: Option<MediaId>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct WebhookUpdate {
    pub channel_id: Option<ChannelId>,
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub avatar: Option<Option<MediaId>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub rotate_token: bool,
}
