use std::{sync::Arc, time::Duration};

use dashmap::DashMap;
use tokio::task::JoinHandle;
use tracing::{debug, error};
use types::user_status::Status;
use types::user_status::StatusType;
use types::User;
use types::UserId;

use crate::error::Error;
use crate::Result;
use crate::ServerStateInner;

pub struct ServiceUserStatus {
    state: Arc<ServerStateInner>,
    online: DashMap<UserId, OnlineState>,
}

struct OnlineState {
    expire_handle: JoinHandle<()>,
}

impl ServiceUserStatus {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            online: DashMap::new(),
        }
    }

    pub async fn ping(&self, user_id: UserId) -> Result<User> {
        let s = self.state.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(60)).await;
            s.services().user_status.online.remove(&user_id);
            debug!("expire!");
        });
        self.online.insert(
            user_id,
            OnlineState {
                expire_handle: handle,
            },
        );
        let data = self.state.data();
        let mut user = data.user_get(user_id).await?;
        user.status = Status {
            status: StatusType::Online,
        };
        // self.state.broadcast(types::MessageSync::UpsertUser { user: () });
        Ok(user)
    }

    pub fn get(&self, user_id: UserId) -> Status {
        if self.online.contains_key(&user_id) {
            Status {
                status: StatusType::Online,
            }
        } else {
            Status {
                status: StatusType::Offline,
            }
        }
    }
}
