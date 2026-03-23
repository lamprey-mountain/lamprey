use std::sync::Arc;

use common::v1::types::ConnectionId;
use dashmap::DashMap;

use crate::{sync::Connection, ServerStateInner};

pub struct ServiceConnections {
    _state: Arc<ServerStateInner>,

    // TODO(#997): limit number of connections per user, clean up old/unused entries
    pub live: DashMap<ConnectionId, Connection>,
}

impl ServiceConnections {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            _state: state,
            live: DashMap::new(),
        }
    }
}
