#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::webhook::Webhook;
use crate::v1::types::{ChannelId, RoomId, WebhookId};

/// something happened with a webhook
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DispatchWebhook {
    pub webhook_id: WebhookId,

    /// the room this webhook belongs to, if any
    pub room_id: Option<RoomId>,

    /// the channel this webhook belongs to
    pub channel_id: ChannelId,

    #[serde(flatten)]
    pub inner: DispatchWebhookInner,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum DispatchWebhookInner {
    /// a webhook was created
    WebhookCreate { webhook: Box<Webhook> },

    /// a webhook was updated
    WebhookUpdate { webhook: Box<Webhook> },

    /// a webhook was deleted
    WebhookDelete,
}
