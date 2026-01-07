use std::sync::Arc;

use common::v1::types::{ChannelId, MessageId, NotificationId};
use serde::Serialize;

use crate::{Result, ServerStateInner};

pub struct ServiceNotifications {
    state: Arc<ServerStateInner>,
}

/// payload sent via web push api
///
/// since the web push api has a pretty low payload size, generally around 2048
/// bytes, this is mostly a "wake up" notif. the client will fetch the full data
/// when receiving this.
#[derive(Debug, Serialize)]
pub struct NotificationPayload {
    pub id: NotificationId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
}

impl ServiceNotifications {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { state }
    }

    /// send a notification to a user through the web push api
    pub async fn push(&self, payload: NotificationPayload) -> Result<()> {
        todo!()
    }
}
