use common::v1::types::{ConnectionId, SessionToken, SyncParams, SyncResume, presence::Presence};
use dashmap::DashMap;

use crate::{
    prelude::*,
    sync::next::{Connection2, ConnectionHandle},
};

// TODO(#997): limit number of connections per user, clean up old/unused entries
pub struct ServiceConnections {
    globals: Globals,
    connections: DashMap<ConnectionId, ConnectionHandle>,
}

// TODO: make common use Hello for MessageClient::Hello
pub struct Hello {
    pub token: SessionToken,
    pub presence: Option<Presence>,
    pub resume: Option<SyncResume>,
}

// TODO: supervise connection actors/tasks
impl ServiceConnections {
    pub fn new(globals: Globals) -> Self {
        Self {
            globals,
            connections: DashMap::new(),
        }
    }

    /// spawn or resume a connection.
    pub async fn accept(&self, hello: Hello) -> Result<ConnectionHandle> {
        let session = self
            .globals
            .services()
            .sessions
            .get_by_token(hello.token)
            .await?;

        if let Some(resume) = hello.resume {
            // get existing connection (handle)
            // update transport
            // update seq (rewind)
            todo!()
        } else {
            let handle = Connection2::create(self.globals.clone(), session);
            self.connections.insert(handle.id(), handle.clone());
            Ok(handle)
        }
    }

    /// get a connection actor handle from its connection id
    pub fn get(&self, id: ConnectionId) -> Option<ConnectionHandle> {
        self.connections.get(&id).map(|r| r.value().clone())
    }
}
