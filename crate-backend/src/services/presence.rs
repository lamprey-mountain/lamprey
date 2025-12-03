use std::{sync::Arc, time::Duration};

use common::v1::types::presence::{Presence, Status};
use common::v1::types::MessageSync;
use common::v1::types::{User, UserId};
use dashmap::DashMap;
use tokio::task::JoinHandle;
use tracing::debug;

use crate::{Result, ServerStateInner};

/// when to expire presences from disconnected users
// currently relies on sync heartbeat time
// TODO: expire presence faster on sync websocket disconnect
const PRESENCE_EXPIRE: Duration = Duration::from_secs(40);

pub struct ServicePresence {
    state: Arc<ServerStateInner>,
    presences: DashMap<UserId, OnlineState>,
}

struct OnlineState {
    expire_handle: JoinHandle<Result<()>>,
    presence: Presence,
}

impl ServicePresence {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            presences: DashMap::new(),
        }
    }

    /// keep the status for a user alive
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn ping(&self, user_id: UserId) -> Result<User> {
        match self.presences.remove(&user_id) {
            Some((_, s)) => {
                s.expire_handle.abort();
                self.set_inner(user_id, s.presence, true).await
            }
            None => self.set_inner(user_id, Presence::offline(), false).await,
        }
    }

    /// set the status for a user
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn set(&self, user_id: UserId, status: Presence) -> Result<User> {
        self.set_inner(user_id, status, false).await
    }

    /// set the status for a user, with a longer expiration
    ///
    /// this is for manual status updates, not presence-based ones
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn set_manual(&self, user_id: UserId, status: Presence) -> Result<User> {
        self.set_inner_expire(user_id, status, false, Duration::from_secs(60 * 5))
            .await
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn set_inner(
        &self,
        user_id: UserId,
        status: Presence,
        skip_broadcast: bool,
    ) -> Result<User> {
        self.set_inner_expire(user_id, status, skip_broadcast, PRESENCE_EXPIRE)
            .await
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn set_inner_expire(
        &self,
        user_id: UserId,
        status: Presence,
        skip_broadcast: bool,
        expire: Duration,
    ) -> Result<User> {
        let s = self.state.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(expire).await;
            let had = s.services().presence.presences.remove(&user_id);
            debug!(
                "expire status for {user_id}, had {:?}",
                had.as_ref().map(|h| &h.1.presence)
            );
            if had.is_none_or(|(_, s)| s.presence != Presence::offline()) {
                s.broadcast(MessageSync::PresenceUpdate {
                    user_id,
                    presence: Presence::offline(),
                })?;
            }
            Result::Ok(())
        });

        let old = self.presences.insert(
            user_id,
            OnlineState {
                expire_handle: handle,
                presence: status.clone(),
            },
        );

        if let Some(old) = &old {
            old.expire_handle.abort();
        }

        let srv = self.state.services();
        let user = srv.users.get(user_id, None).await?;

        if old.is_none_or(|s| s.presence != status) && !skip_broadcast {
            self.state.broadcast(MessageSync::PresenceUpdate {
                user_id,
                presence: status.clone(),
            })?;
        }

        Ok(user)
    }

    /// get the presence for a user
    pub fn get(&self, user_id: UserId) -> Presence {
        if let Some(s) = self.presences.get(&user_id) {
            s.presence.clone()
        } else {
            Presence::offline()
        }
    }

    pub fn is_online(&self, user_id: UserId) -> bool {
        self.presences
            .get(&user_id)
            .map(|s| s.presence.status != Status::Offline)
            .unwrap_or(false)
    }
}
