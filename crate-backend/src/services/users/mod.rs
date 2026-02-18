#![allow(unused)] // TEMP: suppress warnings here for now

use std::cmp::Ordering;
use std::sync::Arc;

use common::v1::types::{Channel, Permission, ThreadMemberPut};
use common::v1::types::{User, UserId};
use dashmap::DashMap;
use tracing::debug;

use crate::types::{DbChannelCreate, DbChannelType};
use crate::{Error, Result, ServerStateInner};

mod affinity;

pub struct ServiceUsers {
    state: Arc<ServerStateInner>,
    dm_lock: DashMap<(UserId, UserId), ()>,
}

/// an identifier for a dm channel
pub struct DmKey(UserId, UserId);
impl DmKey {
    /// create a new dm key, automatically sorting user ids
    pub fn new(_a: UserId, _b: UserId) -> Self {
        todo!()
    }

    pub fn get_users(&self) -> (UserId, UserId) {
        todo!()
    }
}

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
            usr.user_config = Some(
                self.state
                    .services()
                    .cache
                    .user_config_user_get(viewer_id, user_id)
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
            if let Some(user) = srv.cache.users.get(user_id).await {
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
        let thread = srv.channels.get(thread_id, Some(user_id)).await?;
        Ok((thread, true))
    }

    /// add private user data to each user
    pub async fn populate_private(&self, _users: &mut [User], _user_id: UserId) -> Result<()> {
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
