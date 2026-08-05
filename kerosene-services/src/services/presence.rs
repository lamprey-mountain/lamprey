use std::collections::HashMap;
use std::time::Duration;

use common::v1::types::MessageSync;
use common::v1::types::presence::{Presence, Status};
use common::v1::types::{ConnectionId, User, UserId};
use dashmap::DashMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::time::{DelayQueue, delay_queue};
use tracing::debug;

use crate::prelude::*;

/// when to expire presences from disconnected users
// currently relies on sync heartbeat time
// TODO: expire presence faster on sync websocket disconnect
const PRESENCE_EXPIRE: Duration = Duration::from_secs(40);

/// when to expire manually set presences
const PRESENCE_EXPIRE_MANUAL: Duration = Duration::from_secs(60 * 5);

pub struct ServicePresence {
    state: Globals,
    presences: Arc<DashMap<UserId, UserPresences>>,
    cmd_tx: mpsc::UnboundedSender<PresenceEvent>,
}

#[derive(Debug, Default)]
struct UserPresences {
    by_conn: HashMap<ConnectionId, Presence>,
    manual: Option<Presence>,
}

impl UserPresences {
    fn is_empty(&self) -> bool {
        self.by_conn.is_empty() && self.manual.is_none()
    }

    fn resolve(&self) -> Presence {
        // manually set presence overrides everything
        if let Some(p) = &self.manual {
            return p.clone();
        }

        Presence {
            status: self
                .by_conn
                .values()
                .fold(Status::Offline, |s, p| s.max(p.status)),
            activities: self
                .by_conn
                .values()
                .flat_map(|p| p.activities.iter())
                .cloned()
                .collect(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum ExpireKey {
    Connection(UserId, ConnectionId),
    Manual(UserId),
}

enum PresenceEvent {
    ConnectionSet(ConnectionId, UserId, Presence),
    Heartbeat(ConnectionId, UserId),
    Disconnected(ConnectionId, UserId),
    ManuallySet(UserId, Presence),
}

struct PresenceExpiryActor {
    state: Globals,
    presences: Arc<DashMap<UserId, UserPresences>>,
    queue: DelayQueue<ExpireKey>,
    conn_keys: HashMap<(UserId, ConnectionId), delay_queue::Key>,
    manual_keys: HashMap<UserId, delay_queue::Key>,
}

impl PresenceExpiryActor {
    async fn run(mut self, mut cmd_rx: mpsc::UnboundedReceiver<PresenceEvent>) {
        loop {
            tokio::select! {
                event = cmd_rx.recv() => {
                    match event {
                        Some(event) => self.handle_event(event),
                        None => break, // ServicePresence dropped
                    }
                }
                // guard required: polling an empty DelayQueue returns
                // Ready(None) instead of Pending, which would busy-spin
                Some(expired) = self.queue.next(), if !self.queue.is_empty() => {
                    self.handle_timeout(expired.into_inner());
                }
            }
        }
    }

    fn handle_event(&mut self, event: PresenceEvent) {
        match event {
            PresenceEvent::ConnectionSet(conn_id, user_id, presence) => {
                let old = self.presences.get(&user_id).map(|p| p.resolve());
                self.presences
                    .entry(user_id)
                    .or_default()
                    .by_conn
                    .insert(conn_id, presence);
                self.schedule_connection(user_id, conn_id);
                self.broadcast_if_changed(user_id, old);
            }

            PresenceEvent::Heartbeat(conn_id, user_id) => {
                // only refresh if the connection actually has presence set;
                // a stray ping shouldn't conjure a user online
                let known = self
                    .presences
                    .get(&user_id)
                    .is_some_and(|p| p.by_conn.contains_key(&conn_id));
                if known {
                    self.schedule_connection(user_id, conn_id);
                }
            }

            PresenceEvent::Disconnected(conn_id, user_id) => {
                self.remove_connection(user_id, conn_id);
            }

            PresenceEvent::ManuallySet(user_id, presence) => {
                let old = self.presences.get(&user_id).map(|p| p.resolve());
                self.presences.entry(user_id).or_default().manual = Some(presence);
                self.schedule_manual(user_id);
                self.broadcast_if_changed(user_id, old);
            }
        }
    }

    fn handle_timeout(&mut self, key: ExpireKey) {
        match key {
            ExpireKey::Connection(user_id, conn_id) => {
                debug!("connection {conn_id:?} timed out for {user_id}");
                self.conn_keys.remove(&(user_id, conn_id));
                self.remove_connection(user_id, conn_id);
            }
            ExpireKey::Manual(user_id) => {
                debug!("manual presence expired for {user_id}");
                self.manual_keys.remove(&user_id);
                let old = self.presences.get(&user_id).map(|p| p.resolve());
                if let Some(mut entry) = self.presences.get_mut(&user_id) {
                    entry.manual = None;
                }
                self.prune_if_empty(user_id);
                self.broadcast_if_changed(user_id, old);
            }
        }
    }

    fn remove_connection(&mut self, user_id: UserId, conn_id: ConnectionId) {
        if let Some(key) = self.conn_keys.remove(&(user_id, conn_id)) {
            self.queue.try_remove(&key);
        }
        let old = self.presences.get(&user_id).map(|p| p.resolve());
        if let Some(mut entry) = self.presences.get_mut(&user_id) {
            entry.by_conn.remove(&conn_id);
        }
        self.prune_if_empty(user_id);
        self.broadcast_if_changed(user_id, old);
    }

    /// remove empty map entry
    fn prune_if_empty(&mut self, user_id: UserId) {
        let empty = self.presences.get(&user_id).is_some_and(|p| p.is_empty());
        if empty {
            self.presences.remove(&user_id);
            if let Some(key) = self.manual_keys.remove(&user_id) {
                self.queue.try_remove(&key);
            }
        }
    }

    fn schedule_connection(&mut self, user_id: UserId, conn_id: ConnectionId) {
        let k = (user_id, conn_id);
        if let Some(key) = self.conn_keys.get(&k) {
            self.queue.reset(key, PRESENCE_EXPIRE);
        } else {
            let key = self
                .queue
                .insert(ExpireKey::Connection(user_id, conn_id), PRESENCE_EXPIRE);
            self.conn_keys.insert(k, key);
        }
    }

    fn schedule_manual(&mut self, user_id: UserId) {
        if let Some(key) = self.manual_keys.get(&user_id) {
            self.queue.reset(key, PRESENCE_EXPIRE_MANUAL);
        } else {
            let key = self
                .queue
                .insert(ExpireKey::Manual(user_id), PRESENCE_EXPIRE_MANUAL);
            self.manual_keys.insert(user_id, key);
        }
    }

    fn broadcast_if_changed(&self, user_id: UserId, old: Option<Presence>) {
        let new = self
            .presences
            .get(&user_id)
            .map(|p| p.resolve())
            .unwrap_or_else(Presence::offline);
        if old.as_ref() != Some(&new) {
            let state = self.state.clone();
            tokio::spawn(async move {
                let _ = state
                    .messaging()
                    .broadcast_global(MessageSync::PresenceUpdate {
                        user_id,
                        presence: new,
                    })
                    .await;
            });
        }
    }
}

impl ServicePresence {
    pub fn new(state: Globals) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let presences = Arc::new(DashMap::new());
        let me = Self {
            state: state.clone(),
            presences: presences.clone(),
            cmd_tx,
        };
        tokio::spawn(
            PresenceExpiryActor {
                state,
                presences,
                queue: DelayQueue::new(),
                conn_keys: HashMap::new(),
                manual_keys: HashMap::new(),
            }
            .run(cmd_rx),
        );
        me
    }

    /// keep the presence for a user alive
    ///
    /// does nothing if there is no presence
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn ping(&self, connection_id: ConnectionId, user_id: UserId) {
        let _ = self
            .cmd_tx
            .send(PresenceEvent::Heartbeat(connection_id, user_id));
    }

    /// mark a connection as being disconnected
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn disconnect(&self, connection_id: ConnectionId, user_id: UserId) {
        let _ = self
            .cmd_tx
            .send(PresenceEvent::Disconnected(connection_id, user_id));
    }

    /// set a user's presence from a connection
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn set(&self, connection_id: ConnectionId, user_id: UserId, presence: Presence) {
        let _ = self.cmd_tx.send(PresenceEvent::ConnectionSet(
            connection_id,
            user_id,
            presence,
        ));
    }

    /// set a user's presence from the manual set presence endpoint
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn set_manually(&self, user_id: UserId, presence: Presence) {
        let _ = self
            .cmd_tx
            .send(PresenceEvent::ManuallySet(user_id, presence));
    }

    /// get the presence for a user
    pub fn get(&self, user_id: UserId) -> Presence {
        self.presences
            .get(&user_id)
            .map(|p| p.resolve())
            .unwrap_or_else(Presence::offline)
    }
}
