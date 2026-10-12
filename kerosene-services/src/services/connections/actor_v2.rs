use common::v1::types::{SessionId, UserId};
use common::v2::types::ConnectionId;
use std::sync::Arc;

use crate::prelude::*;

#[derive(Debug)]
pub struct ConnectionV2 {
    id: ConnectionId,
    session_id: SessionId,
    user_id: Option<UserId>,
}

#[derive(Clone, Debug)]
pub struct ConnectionHandleV2 {
    inner: Arc<ConnectionV2>,
}

impl ConnectionHandleV2 {
    pub fn create(session_id: SessionId, user_id: Option<UserId>) -> Self {
        let id = ConnectionId::new();

        // let queue = ConnectionQueue::new(MAX_QUEUE_LEN);

        let inner = Arc::new(ConnectionV2 {
            id,
            session_id,
            user_id,
        });

        // tokio::spawn(
        //     async move {
        //         conn.spawn().await;
        //     }
        //     .instrument(tracing::debug_span!("connection", id = %id)),
        // );

        Self { inner }
    }

    pub fn id(&self) -> ConnectionId {
        self.inner.id
    }
}
