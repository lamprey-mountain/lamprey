use std::cmp::Ordering;
use std::{sync::Arc, time::Duration};

use common::v1::types::user_status::Status;
use common::v1::types::voice::{SfuCommand, SfuPermissions, VoiceState};
use common::v1::types::{Bot, BotAccess, MessageSync, Thread, ThreadId, ThreadMemberPut};
use common::v1::types::{User, UserId};
use dashmap::DashMap;
use moka::future::Cache;
use tokio::task::JoinHandle;
use tracing::{debug, error};

use crate::types::{DbThreadCreate, DbThreadType};
use crate::{Error, Result, ServerStateInner};

// currently relies on sync heartbeat time
const STATUS_EXPIRE: Duration = Duration::from_secs(40);

pub struct ServiceUsers {
    state: Arc<ServerStateInner>,
    cache_users: Cache<UserId, User>,
    statuses: DashMap<UserId, OnlineState>,
    dm_lock: DashMap<(UserId, UserId), ()>,
    voice_states: DashMap<UserId, VoiceState>,
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
            dm_lock: DashMap::new(),
            voice_states: DashMap::new(),
        }
    }

    pub async fn get(&self, user_id: UserId, viewer_id: Option<UserId>) -> Result<User> {
        let mut usr = self
            .cache_users
            .try_get_with(user_id, self.state.data().user_get(user_id))
            .await
            .map_err(|err| err.fake_clone())?;

        if let Some(viewer_id) = viewer_id {
            usr.user_config = Some(
                self.state
                    .data()
                    .user_config_user_get(viewer_id, user_id)
                    .await?,
            );
        }

        let app = self
            .state
            .data()
            .application_get(user_id.into_inner().into())
            .await
            .ok();
        if let Some(app) = app {
            usr.bot = Some(Bot {
                owner_id: app.owner_id,
                access: if app.public {
                    BotAccess::Public {
                        is_discoverable: false,
                    }
                } else {
                    BotAccess::Private
                },
                is_bridge: app.bridge,
            });
        }
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
                s.broadcast(MessageSync::UserUpdate { user: user.clone() })?;
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
        let user = srv.users.get(user_id, None).await?;

        if old.is_none_or(|s| s.status != status) && !skip_broadcast {
            self.state
                .broadcast(MessageSync::UserUpdate { user: user.clone() })?;
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

    pub fn voice_state_put(&self, state: VoiceState) {
        self.voice_states.insert(state.user_id, state);
    }

    pub fn voice_state_remove(&self, user_id: &UserId) {
        self.voice_states.remove(user_id);
    }

    pub fn voice_state_get(&self, user_id: UserId) -> Option<VoiceState> {
        self.voice_states.get(&user_id).map(|s| s.to_owned())
    }

    pub fn voice_states_list(&self) -> Vec<VoiceState> {
        self.voice_states
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    pub async fn init_dm(&self, user_id: UserId, other_id: UserId) -> Result<(Thread, bool)> {
        let (user_id, other_id) = ensure_dm_canonical(user_id, other_id)?;
        let data = self.state.data();
        let srv = self.state.services();
        let _lock = self.dm_lock.entry((user_id, other_id)).or_default();
        if let Some(thread_id) = data.dm_get(user_id, other_id).await? {
            debug!("dm thread id {thread_id}");
            let thread = srv.threads.get(thread_id, Some(user_id)).await?;
            data.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
                .await?;
            data.thread_member_put(thread_id, other_id, ThreadMemberPut::default())
                .await?;
            return Ok((thread, false));
        }
        let thread_id = data
            .thread_create(DbThreadCreate {
                room_id: None,
                creator_id: user_id,
                name: "dm".to_string(),
                description: None,
                ty: DbThreadType::Dm,
                nsfw: false,
                bitrate: None,
                user_limit: None,
                parent_id: None,
                owner_id: None,
            })
            .await?;
        data.dm_put(user_id, other_id, thread_id).await?;
        data.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
            .await?;
        data.thread_member_put(thread_id, other_id, ThreadMemberPut::default())
            .await?;
        let thread = srv.threads.get(thread_id, Some(user_id)).await?;
        Ok((thread, true))
    }

    pub fn disconnect_everyone_from_thread(&self, thread_id: ThreadId) -> Result<()> {
        for s in &self.voice_states {
            if s.thread_id == thread_id {
                let r = self.state.sushi_sfu.send(SfuCommand::VoiceState {
                    user_id: s.user_id,
                    state: None,
                    permissions: SfuPermissions {
                        speak: false,
                        video: false,
                        priority: false,
                    },
                });
                if let Err(err) = r {
                    error!("failed to disconnect user from thread: {err}");
                }
            }
        }
        self.voice_states.retain(|_, s| s.thread_id != thread_id);
        Ok(())
    }
}

fn ensure_dm_canonical(a: UserId, b: UserId) -> Result<(UserId, UserId)> {
    match a.cmp(&b) {
        Ordering::Less => Ok((a, b)),
        Ordering::Equal => Err(Error::BadStatic("cant dm yourself")),
        Ordering::Greater => Ok((b, a)),
    }
}
