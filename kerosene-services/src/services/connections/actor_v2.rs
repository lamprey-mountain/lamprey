use common::v1::types::{SessionId, UserId};
use common::v2::types::ConnectionId;
use std::sync::Arc;

use crate::prelude::*;

// A simpler Connection state for V2
#[derive(Debug)]
pub struct ConnectionV2 {
    pub id: ConnectionId,
    pub session_id: SessionId,
    pub user_id: Option<UserId>,
}

#[derive(Clone, Debug)]
pub struct ConnectionHandleV2 {
    pub inner: Arc<ConnectionV2>,
}

impl ConnectionHandleV2 {
    pub fn create(session_id: SessionId, user_id: Option<UserId>) -> Self {
        let id = ConnectionId::new();

        let inner = Arc::new(ConnectionV2 {
            id,
            session_id,
            user_id,
        });

        Self { inner }
    }
}

