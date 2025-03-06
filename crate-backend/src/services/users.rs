use std::{sync::Arc, time::Duration};

use dashmap::DashMap;
use moka::future::Cache;
use tokio::task::JoinHandle;
use tracing::debug;
use types::user_status::Status;
use types::MessageSync;
use types::{User, UserId};

use crate::{Result, ServerStateInner};

// currently relies on sync heartbeat time
const STATUS_EXPIRE: Duration = Duration::from_secs(40);

pub struct ServiceUsers {
    state: Arc<ServerStateInner>,
    cache_users: Cache<UserId, User>,
    statuses: DashMap<UserId, OnlineState>,
}

struct OnlineState {
    expire_handle: JoinHandle<Result<()>>,
    status: Status,
}

impl ServiceUsers {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            cache_users: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            statuses: DashMap::new(),
        }
    }

    pub async fn get(&self, user_id: UserId) -> Result<User> {
        let mut usr = self
            .cache_users
            .try_get_with(user_id, self.state.data().user_get(user_id))
            .await
            .map_err(|err| err.fake_clone())?;
        let status = self.status_get(user_id);
        usr.status = status;
        Ok(usr)
    }

    pub async fn invalidate(&self, user_id: UserId) {
        self.cache_users.invalidate(&user_id).await
    }

    /// keep the status for a user alive
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn status_ping(&self, user_id: UserId) -> Result<User> {
        match self.statuses.remove(&user_id) {
            Some((_, s)) => {
                s.expire_handle.abort();
                self.status_set_inner(user_id, s.status, true).await
            }
            None => {
                self.status_set_inner(user_id, Status::offline(), false)
                    .await
            }
        }
    }

    /// set the status for a user
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn status_set(&self, user_id: UserId, status: Status) -> Result<User> {
        self.status_set_inner(user_id, status, false).await
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn status_set_inner(
        &self,
        user_id: UserId,
        status: Status,
        skip_broadcast: bool,
    ) -> Result<User> {
        let s = self.state.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(STATUS_EXPIRE).await;
            let had = s.services().users.statuses.remove(&user_id);
            debug!(
                "expire status for {user_id}, had {:?}",
                had.as_ref().map(|h| &h.1.status)
            );
            if had.is_none_or(|(_, s)| s.status != Status::offline()) {
                let data = s.data();
                let mut user = data.user_get(user_id).await?;
                user.status = Status::offline();
                s.broadcast(MessageSync::UpsertUser { user: user.clone() })?;
            }
            Result::Ok(())
        });

        let old = self.statuses.insert(
            user_id,
            OnlineState {
                expire_handle: handle,
                status: status.clone(),
            },
        );

        if let Some(old) = &old {
            old.expire_handle.abort();
        }

        let srv = self.state.services();
        let user = srv.users.get(user_id).await?;

        if old.is_none_or(|s| s.status != status) && !skip_broadcast {
            self.state
                .broadcast(MessageSync::UpsertUser { user: user.clone() })?;
        }

        Ok(user)
    }

    /// get the status for a user
    pub fn status_get(&self, user_id: UserId) -> Status {
        if let Some(s) = self.statuses.get(&user_id) {
            s.status.clone()
        } else {
            Status::offline()
        }
    }
}
