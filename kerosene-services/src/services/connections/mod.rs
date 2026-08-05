use common::v1::types::{
    ConnectionId, MessageHello,
    presence::{Presence, Status},
};
use dashmap::DashMap;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::time::{DelayQueue, delay_queue};

use crate::{
    prelude::*,
    services::connections::actor::{Connection, ConnectionHandle},
};

const CONNECTION_RESUME_PERIOD: Duration = Duration::from_secs(60);

mod actor;
mod subscriptions;

enum ConnectionEvent {
    Disconnected(ConnectionId),
    Attached(ConnectionId),
}

struct ConnectionExpiryActor {
    connections: Arc<DashMap<ConnectionId, ConnectionHandle>>,
    queue: DelayQueue<ConnectionId>,
    keys: HashMap<ConnectionId, delay_queue::Key>,
}

impl ConnectionExpiryActor {
    async fn run(mut self, mut cmd_rx: mpsc::UnboundedReceiver<ConnectionEvent>) {
        loop {
            tokio::select! {
                event = cmd_rx.recv() => {
                    match event {
                        Some(ConnectionEvent::Disconnected(id)) => {
                            let key = self.queue.insert(id, CONNECTION_RESUME_PERIOD);
                            self.keys.insert(id, key);
                        }
                        Some(ConnectionEvent::Attached(id)) => {
                            if let Some(key) = self.keys.remove(&id) {
                                self.queue.try_remove(&key);
                            }
                        }
                        None => break,
                    }
                }
                Some(expired) = self.queue.next(), if !self.queue.is_empty() => {
                    let id = expired.into_inner();
                    self.keys.remove(&id);
                    self.connections.remove(&id);
                }
            }
        }
    }
}

// TODO(#997): limit number of connections per user, ~~clean up old/unused connections~~
// NOTE: im unsure if limiting number of conns is even worthwhile, they aren't
// that expensive. calculating and sending Ambient is though, so it would
// probably be better to ratelimit new connections than limit the total number
// of connections anyways?
pub struct ServiceConnections {
    globals: Globals,
    connections: Arc<DashMap<ConnectionId, ConnectionHandle>>,
    cmd_tx: mpsc::UnboundedSender<ConnectionEvent>,
    // tasks: tokio::task::JoinSet<(ConnectionId, Result<()>)>,
    // user_connection_counts: DashMap<UserId, usize>,
}

impl ServiceConnections {
    pub fn new(globals: Globals) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let connections = Arc::new(DashMap::new());

        tokio::spawn(
            ConnectionExpiryActor {
                connections: connections.clone(),
                queue: DelayQueue::new(),
                keys: HashMap::new(),
            }
            .run(cmd_rx),
        );

        Self {
            globals,
            connections,
            cmd_tx,
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

    fn schedule_cleanup(&self, id: ConnectionId) {
        let _ = self.cmd_tx.send(ConnectionEvent::Disconnected(id));
    }

    fn cancel_cleanup(&self, id: ConnectionId) {
        let _ = self.cmd_tx.send(ConnectionEvent::Attached(id));
    }
}
