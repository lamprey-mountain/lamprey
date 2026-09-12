use common::v1::types::federation::Remote;
use common::v1::types::{Channel, Permission, ThreadMemberPut};
use common::v1::types::{User, UserId};
use dashmap::DashMap;
use moka::future::Cache;
use tokio::sync::Mutex;
use tracing::debug;

use crate::prelude::*;
use crate::services::users::util::DmKey;
use crate::types::{DbChannelCreate, DbChannelType};

mod affinity;
mod util;

pub struct ServiceUsers {
    globals: Globals,
    dm_lock: DashMap<DmKey, Arc<Mutex<()>>>,
    // TODO: make this not pub
    pub(crate) cache: Cache<UserId, Arc<User>>,
}

// TODO: make services federation aware?
impl ServiceUsers {
    pub fn new(globals: Globals) -> Self {
        Self {
            globals,
            dm_lock: DashMap::new(),
            cache: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
        }
    }

    pub async fn get(&self, user_id: UserId, viewer_id: Option<UserId>) -> Result<User> {
        let usr = self
            .cache
            .try_get_with(user_id, async {
                let user = self.globals.begin_read().await?.user_get(user_id).await?;
                Result::<Arc<User>>::Ok(Arc::new(user))
            })
            .await
            .map_err(|e| e.fake_clone())?;
        // PERF: don't clone
        let mut usr = (*usr).clone();

        let srv = self.globals.services();
        if let Some(viewer_id) = viewer_id {
            usr.preferences = Some(srv.cache.preferences_user_get(viewer_id, user_id).await?);

            let perms = self.globals.services().perms.for_server(viewer_id).await;
            // NOTE: do i want to reveal email addrs to people with UserManage?
            let is_admin = perms.is_ok_and(|p| p.has(Permission::Admin));

            if viewer_id == user_id || is_admin {
                let mut data = self.globals.begin_read().await?;
                usr.emails = Some(data.user_email_list(user_id).await?);
            }
        }

        // FIXME: populate has_mfa

        let status = srv.presence.get(user_id);
        usr.presence = status;
        Ok(usr)
    }

    pub async fn get_many(&self, user_ids: &[UserId]) -> Result<Vec<User>> {
        if user_ids.is_empty() {
            return Ok(vec![]);
        }

        let srv = self.globals.services();
        let mut out = Vec::with_capacity(user_ids.len());
        let mut missing = Vec::new();

        for user_id in user_ids {
            if let Some(user) = self.cache.get(user_id).await {
                let mut user = (*user).clone();
                user.presence = srv.presence.get(*user_id);
                out.push(user);
            } else {
                missing.push(*user_id);
            }
        }

        if !missing.is_empty() {
            let mut txn = self.globals.begin_read().await?;
            let mut users = txn.user_get_many(&missing).await?;
            drop(txn);
            for mut user in users {
                user.presence = srv.presence.get(user.id);
                self.cache.insert(user.id, Arc::new(user.clone())).await;
                out.push(user);
            }
        }

        Ok(out)
    }

    /// lookup a user from a `Remote` (TODO)
    pub async fn get_remote(&self, _remote: Remote<UserId>) -> Result<User> {
        todo!()
    }

    pub async fn invalidate(&self, user_id: UserId) {
        self.cache.invalidate(&user_id).await
    }

    pub fn purge_cache(&self) {
        self.cache.invalidate_all();
    }

    pub async fn init_dm(
        &self,
        user_id: UserId,
        other_id: UserId,
        locked: bool,
    ) -> Result<(Channel, bool)> {
        let key = DmKey::new(user_id, other_id)?;
        let (user_id, other_id) = key.get_users();

        let lock = self
            .dm_lock
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _guard = lock.lock().await;

        let srv = self.globals.services();
        let mut txn = self.globals.begin().await?;
        if let Some(thread_id) = txn.dm_get(user_id, other_id).await? {
            debug!("dm thread id {thread_id}");
            let chan = srv.channels.get(thread_id, Some(user_id)).await?;
            txn.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
                .await?;
            txn.thread_member_put(thread_id, other_id, ThreadMemberPut::default())
                .await?;
            txn.commit().await?;
            return Ok((chan, false));
        }
        let thread_id = txn
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
        txn.dm_put(user_id, other_id, thread_id).await?;
        txn.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
            .await?;
        txn.thread_member_put(thread_id, other_id, ThreadMemberPut::default())
            .await?;
        txn.commit().await?;
        let chan = srv.channels.get(thread_id, Some(user_id)).await?;
        Ok((chan, true))
    }

    /// add private user data to each user (TODO)
    pub async fn populate_private(&self, _users: &mut [User], _user_id: UserId) -> Result<()> {
        Ok(())
    }
}

// PERF: use a compact struct for user data, don't automatically clone and include presence everywhere
// then maybe have struct User<'a> { field: &'a String } for serialization, instead of cloning into owned User
// pub struct UserData {
//     pub version_id: UserVerId,
//     pub name: String,
//     pub description: Option<String>,
//     pub avatar: Option<MediaId>,
//     pub banner: Option<MediaId>,
//     pub bot: bool,
//     pub system: bool,
//     pub puppet: Option<Puppet>,
//     pub webhook: Option<UserWebhook>,
//     pub suspended: Option<Suspended>,
//     pub registered_at: Option<Time>,
//     pub deleted_at: Option<Time>,
//     pub emails: Vec<EmailInfo>,
//     pub has_mfa: bool,
//     pub remote: Option<Remote<UserId>>,
// }
