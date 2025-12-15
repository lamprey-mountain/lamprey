use std::cmp::Ordering;
use std::sync::Arc;

use common::v1::types::{Channel, Permission, ThreadMemberPut};
use common::v1::types::{User, UserId};
use dashmap::DashMap;
use moka::future::Cache;
use tracing::debug;

use crate::types::{DbChannelCreate, DbChannelType};
use crate::{Error, Result, ServerStateInner};

pub struct ServiceUsers {
    state: Arc<ServerStateInner>,
    cache_users: Cache<UserId, User>,
    dm_lock: DashMap<(UserId, UserId), ()>,
}

impl ServiceUsers {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            cache_users: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            dm_lock: DashMap::new(),
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

            let perms = self
                .state
                .services()
                .perms
                .for_room(viewer_id, common::v1::types::SERVER_ROOM_ID)
                .await;
            let is_admin = perms.is_ok_and(|p| p.has(Permission::Admin));

            if viewer_id == user_id || is_admin {
                usr.emails = Some(self.state.data().user_email_list(user_id).await?);
            }
        }

        let status = self.state.services().presence.get(user_id);
        usr.presence = status;
        Ok(usr)
    }

    pub async fn get_many(&self, user_ids: &[UserId]) -> Result<Vec<User>> {
        if user_ids.is_empty() {
            return Ok(vec![]);
        }
        let mut users = self.state.data().user_get_many(user_ids).await?;
        for user in &mut users {
            user.presence = self.state.services().presence.get(user.id);
        }
        Ok(users)
    }

    pub async fn invalidate(&self, user_id: UserId) {
        self.cache_users.invalidate(&user_id).await
    }

    pub fn purge_cache(&self) {
        self.cache_users.invalidate_all();
    }

    pub async fn init_dm(&self, user_id: UserId, other_id: UserId) -> Result<(Channel, bool)> {
        let (user_id, other_id) = ensure_dm_canonical(user_id, other_id)?;
        let data = self.state.data();
        let srv = self.state.services();
        let _lock = self.dm_lock.entry((user_id, other_id)).or_default();
        if let Some(thread_id) = data.dm_get(user_id, other_id).await? {
            debug!("dm thread id {thread_id}");
            let thread = srv.channels.get(thread_id, Some(user_id)).await?;
            data.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
                .await?;
            data.thread_member_put(thread_id, other_id, ThreadMemberPut::default())
                .await?;
            return Ok((thread, false));
        }
        let thread_id = data
            .channel_create(DbChannelCreate {
                room_id: None,
                creator_id: user_id,
                name: "dm".to_string(),
                description: None,
                ty: DbChannelType::Dm,
                nsfw: false,
                bitrate: None,
                user_limit: None,
                parent_id: None,
                owner_id: None,
                icon: None,
                invitable: false,
                auto_archive_duration: None,
                default_auto_archive_duration: None,
                slowmode_thread: None,
                slowmode_message: None,
                default_slowmode_message: None,
                tags: None,
            })
            .await?;
        data.dm_put(user_id, other_id, thread_id).await?;
        data.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
            .await?;
        data.thread_member_put(thread_id, other_id, ThreadMemberPut::default())
            .await?;
        let thread = srv.channels.get(thread_id, Some(user_id)).await?;
        Ok((thread, true))
    }
}

fn ensure_dm_canonical(a: UserId, b: UserId) -> Result<(UserId, UserId)> {
    match a.cmp(&b) {
        Ordering::Less => Ok((a, b)),
        Ordering::Equal => Err(Error::BadStatic("cant dm yourself")),
        Ordering::Greater => Ok((b, a)),
    }
}
