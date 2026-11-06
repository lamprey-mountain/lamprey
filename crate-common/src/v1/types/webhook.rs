use lamprey_macros::record;

use crate::util::registry::export_models;
use crate::v1::types::{ChannelId, MediaId, RoomId, UserId, WebhookId};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

#[record]
pub struct Webhook {
    pub id: WebhookId,
    pub room_id: Option<RoomId>,
    pub channel_id: ChannelId,
    pub creator_id: Option<UserId>,
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: String,
    pub avatar: Option<MediaId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[record]
pub struct WebhookCreate {
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: String,
    pub avatar: Option<MediaId>,
}

/// webhook info fetched from its token/url
#[record]
pub struct WebhookInfo {
    pub id: WebhookId,
    pub room_id: Option<RoomId>,
    pub channel_id: ChannelId,
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: String,
    pub avatar: Option<MediaId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[record]
pub struct WebhookUpdate {
    pub channel_id: Option<ChannelId>,
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "some_option")]
    pub avatar: Option<Option<MediaId>>,
    #[serde(default)]
    pub rotate_token: bool,
}

impl From<Webhook> for WebhookInfo {
    fn from(webhook: Webhook) -> Self {
        Self {
            id: webhook.id,
            room_id: webhook.room_id,
            channel_id: webhook.channel_id,
            name: webhook.name,
            avatar: webhook.avatar,
            token: webhook.token,
        }
    }
}

export_models!(Webhook, WebhookCreate, WebhookUpdate, WebhookInfo);
