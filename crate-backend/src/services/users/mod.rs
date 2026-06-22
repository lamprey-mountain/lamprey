use std::cmp::Ordering;
use std::sync::Arc;

use common::v1::types::federation::Remote;
use common::v1::types::{Channel, Permission, ThreadMemberPut};
use common::v1::types::{User, UserId};
use dashmap::DashMap;
use tracing::debug;

use crate::services::users::util::DmKey;
use crate::types::{DbChannelCreate, DbChannelType};
use crate::{Error, Result, ServerStateInner};

mod affinity;
mod util;

pub struct ServiceUsers {
    state: Arc<ServerStateInner>,
    dm_lock: DashMap<(UserId, UserId), ()>,
}

// TODO: make services federation aware?
impl ServiceUsers {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            dm_lock: DashMap::new(),
        }
    }

    pub async fn get(&self, user_id: UserId, viewer_id: Option<UserId>) -> Result<User> {
        let mut usr = self.state.services().cache.user_get(user_id).await?;

        if let Some(viewer_id) = viewer_id {
            usr.preferences = Some(
                self.state
                    .services()
                    .cache
                    .preferences_user_get(viewer_id, user_id)
                    .await?,
            );

            let perms = self.state.services().perms.for_server(viewer_id).await;
            // NOTE: do i want to reveal email addrs to people with UserManage?
            let is_admin = perms.is_ok_and(|p| p.has(Permission::Admin));

            if viewer_id == user_id || is_admin {
                usr.emails = Some(self.state.data().user_email_list(user_id).await?);
            }
        }

        // FIXME: populate has_mfa

        let status = self.state.services().presence.get(user_id);
        usr.presence = status;
        Ok(usr)
    }

    pub async fn get_many(&self, user_ids: &[UserId]) -> Result<Vec<User>> {
        if user_ids.is_empty() {
            return Ok(vec![]);
        }

        let srv = self.state.services();
        let mut out = Vec::with_capacity(user_ids.len());
        let mut missing = Vec::new();

        for user_id in user_ids {
            if let Some(mut user) = srv.cache.users.get(user_id).await {
                user.presence = srv.presence.get(*user_id);
                out.push(user);
            } else {
                missing.push(*user_id);
            }
        }

        if !missing.is_empty() {
            let mut users = self.state.data().user_get_many(&missing).await?;
            for user in &mut users {
                user.presence = srv.presence.get(user.id);
                srv.cache.users.insert(user.id, user.clone()).await;
                out.push(user.clone());
            }
        }

        Ok(out)
    }

    /// lookup a user from a `Remote` (TODO)
    pub async fn get_remote(&self, _remote: Remote<UserId>) -> Result<User> {
        todo!()
    }

    pub async fn invalidate(&self, user_id: UserId) {
        self.state.services().cache.user_invalidate(user_id).await
    }

    pub fn purge_cache(&self) {
        self.state.services().cache.user_purge();
    }

    pub async fn init_dm(
        &self,
        user_id: UserId,
        other_id: UserId,
        locked: bool,
    ) -> Result<(Channel, bool)> {
        let (user_id, other_id) = DmKey::new(user_id, other_id)?.get_users();
        let mut data = self.state.acquire_data().await?;
        let srv = self.state.services();
        let _lock = self.dm_lock.entry((user_id, other_id)).or_default();
        if let Some(thread_id) = data.dm_get(user_id, other_id).await? {
            debug!("dm thread id {thread_id}");
            let chan = srv.channels.get(thread_id, Some(user_id)).await?;
            data.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
                .await?;
            data.thread_member_put(thread_id, other_id, ThreadMemberPut::default())
                .await?;
            return Ok((chan, false));
        }
        let thread_id = data
            .channel_create(DbChannelCreate {
                room_id: None,
                creator_id: user_id,
                name: "dm".to_string(),
                description: None,
                url: None,
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
                locked,
                tags: None,
            })
            .await?;
        data.dm_put(user_id, other_id, thread_id).await?;
        data.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
            .await?;
        data.thread_member_put(thread_id, other_id, ThreadMemberPut::default())
            .await?;
        data.commit().await?;
        let chan = srv.channels.get(thread_id, Some(user_id)).await?;
        Ok((chan, true))
    }

    /// add private user data to each user (TODO)
    pub async fn populate_private(&self, _users: &mut [User], _user_id: UserId) -> Result<()> {
        Ok(())
    }
}
