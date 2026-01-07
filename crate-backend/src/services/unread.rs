use std::sync::Arc;

use common::v1::types::{ChannelId, MessageId, MessageVerId, UserId};

use crate::{Result, ServerStateInner};

pub struct ServiceUnread {
    state: Arc<ServerStateInner>,
}

impl ServiceUnread {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { state }
    }

    pub async fn ack(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
        mention_count: u64,
    ) -> Result<()> {
        todo!()
    }
}
