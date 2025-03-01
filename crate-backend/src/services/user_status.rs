use std::{sync::Arc, time::Duration};

use dashmap::DashMap;
use tokio::task::JoinHandle;
use tracing::debug;
use types::user_status::Status;
use types::MessageSync;
use types::User;
use types::UserId;

use crate::Result;
use crate::ServerStateInner;

pub struct ServiceUserStatus {
    state: Arc<ServerStateInner>,
    statuses: DashMap<UserId, OnlineState>,
}

struct OnlineState {
    _expire_handle: JoinHandle<()>,
    status: Status,
}

impl ServiceUserStatus {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            statuses: DashMap::new(),
        }
    }

    /// keep the status for a user alive
    pub async fn ping(&self, user_id: UserId) -> Result<User> {
        match self.statuses.remove(&user_id) {
            Some((_, s)) => self.set_inner(user_id, s.status, true).await,
            None => self.set_inner(user_id, Status::offline(), false).await,
        }
    }

    /// set the status for a user
    pub async fn set(&self, user_id: UserId, status: Status) -> Result<User> {
        self.set_inner(user_id, status, false).await
    }

    async fn set_inner(
        &self,
        user_id: UserId,
        status: Status,
        skip_broadcast: bool,
    ) -> Result<User> {
        let s = self.state.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let had = s.services().user_status.statuses.remove(&user_id);
            debug!(
                "expire status for {user_id}, had {:?}",
                had.map(|h| h.1.status)
            );
        });

        let old = self.statuses.insert(
            user_id,
            OnlineState {
                _expire_handle: handle,
                status: status.clone(),
            },
        );

        let data = self.state.data();
        let mut user = data.user_get(user_id).await?;
        user.status = status.clone();

        if old.is_none_or(|s| s.status != status) && !skip_broadcast {
            self.state
                .broadcast(MessageSync::UpsertUser { user: user.clone() })?;
        }

        Ok(user)
    }

    /// get the status for a user
    pub fn get(&self, user_id: UserId) -> Status {
        if let Some(s) = self.statuses.get(&user_id) {
            s.status.clone()
        } else {
            Status::offline()
        }
    }
}
