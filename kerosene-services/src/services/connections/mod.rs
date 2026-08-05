use common::v1::types::{
    ConnectionId, MessageHello,
    presence::{Presence, Status},
};
use dashmap::DashMap;

use crate::{
    prelude::*,
    services::connections::actor::{Connection, ConnectionHandle},
};

mod actor;
mod subscriptions;

// TODO(#997): limit number of connections per user, clean up old/unused entries
pub struct ServiceConnections {
    globals: Globals,
    connections: DashMap<ConnectionId, ConnectionHandle>,
    // TODO: add for supervision
    // tasks: JoinSet<()>,

    // tasks: tokio::task::JoinSet<(ConnectionId, Result<()>)>,
    // user_connection_counts: DashMap<UserId, usize>,
}

// pub struct ConnectionResult {
//     id: ConnectionId,
//     result: Result<()>,
// }

// TODO: supervise connection actors/tasks
impl ServiceConnections {
    pub fn new(globals: Globals) -> Self {
        Self {
            globals,
            connections: DashMap::new(),
        }
    }

    /// create/spawn a new connection.
    ///
    /// does not handle resumes.
    pub async fn accept(&self, hello: MessageHello) -> Result<ConnectionHandle> {
        let srv = self.globals.services();
        let session = srv.sessions.get_by_token(hello.token).await?;

        let handle = Connection::create(self.globals.clone(), (*session).clone());
        self.connections.insert(handle.id(), handle.clone());

        if let (presence, Some(user_id)) = (hello.presence, session.user_id()) {
            let user = srv.users.get(user_id, Some(user_id)).await?;
            if !user.is_suspended() {
                let presence = presence.unwrap_or(Presence {
                    status: Status::Online,
                    activities: vec![],
                });
                srv.presence.set(handle.id(), user_id, presence);
            }
        }

        Ok(handle)
    }

    /// get a connection actor handle from its connection id
    pub fn get(&self, id: ConnectionId) -> Option<ConnectionHandle> {
        self.connections.get(&id).map(|r| r.value().clone())
    }
}
