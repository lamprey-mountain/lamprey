pub mod actor;
pub mod types;

pub use types::{
    CachedChannel, CachedPermissionOverwrite, CachedRole, CachedRoomMember, CachedThread,
    CleanupIdleLists, EnsureMembers, GetSnapshot, MemberListCommandMsg, MemberListSubscribeMsg,
    RoomData, RoomHandle, RoomSnapshot, RoomUnavailable, RoomUnavailableReason, SyncMessage,
};

pub use actor::RoomActor;

use std::sync::Arc;
use std::time::Duration;

use crate::consts::IDLE_TIMEOUT_ROOM;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::{Changes, Diff};
use common::v1::types::{
    AuditLogEntryStatus, AuditLogEntryType, ChannelId, ChannelType, MessageSync, MessageType,
    RoleId, Room, RoomCreate, RoomId, RoomMemberOrigin, RoomMemberPut, RoomPatch, ThreadMemberPut,
    UserId,
};
use dashmap::{DashMap, DashSet};
use moka::future::Cache;
use validator::Validate;

use crate::consts::MAX_LOADED_ROOMS;
use crate::error::Result;
use crate::routes::util::Auth;
use crate::services::room_template::builtin;
use crate::types::{DbMessageCreate, DbRoomCreate, MediaLinkType};
use crate::{Error, ServerStateInner};

use futures::future::BoxFuture;

#[derive(Clone)]
pub struct ServiceRooms {
    state: Arc<ServerStateInner>,
    idempotency_keys: Cache<String, Room>,
    pub(crate) actors: Cache<RoomId, RoomHandle>,
    /// Keep an in-memory map of UserId -> Set of RoomIds for fast fan-out of presence/user updates
    pub(crate) user_rooms: Arc<DashMap<UserId, DashSet<RoomId>>>,
}

impl ServiceRooms {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            idempotency_keys: Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .build(),
            actors: Cache::builder()
                .max_capacity(MAX_LOADED_ROOMS)
                .time_to_idle(Duration::from_secs(IDLE_TIMEOUT_ROOM))
                .support_invalidation_closures()
                .eviction_listener(|room_id, handle: RoomHandle, cause| {
                    tracing::debug!(?room_id, ?cause, "Evicting room actor");
                    // Use Kameo's kill to stop the actor
                    let _ = handle.actor_ref.kill();
                })
                .build(),
            user_rooms: Arc::new(DashMap::new()),
        }
    }

    /// load a room snapshot, ensuring members are loaded if requested.
    pub fn load_room(
        &self,
        room_id: RoomId,
        ensure_members: bool,
    ) -> BoxFuture<'_, Result<Arc<RoomSnapshot>>> {
        Box::pin(async move {
            let handle = self
                .actors
                .try_get_with(room_id, async {
                    Ok::<RoomHandle, Error>(RoomActor::spawn_room(room_id, self.state.clone()))
                })
                .await
                .map_err(|e| e.fake_clone())?;

            if ensure_members {
                handle
                    .actor_ref
                    .ask(EnsureMembers)
                    .send()
                    .await
                    .map_err(|e| Error::Internal(format!("Actor mailbox closed: {e}")))?;
            }

            let mut rx = handle.snapshot_rx.clone();
            let snapshot = rx
                .wait_for(|s| {
                    if s.is_loading() {
                        return false;
                    }
                    if ensure_members && s.is_without_members() {
                        return false;
                    }
                    true
                })
                .await
                .map_err(|_| Error::ApiError(ApiError::from_code(ErrorCode::UnknownRoom)))?;

            let snapshot = snapshot.clone();

            if snapshot.is_not_found() {
                return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownRoom)));
            }

            if let RoomSnapshot::Unavailable(_) = snapshot.as_ref() {
                // If it's backlogged, we should probably evict it so it can retry later
                return Err(Error::ServiceUnavailable);
            }

            Ok(snapshot)
        })
    }

    /// mark a room as unavailable
    pub async fn mark_unavailable(&self, room_id: RoomId, _reason: RoomUnavailableReason) {
        if let Some(_handle) = self.actors.get(&room_id).await {
            // We can't easily update the watch channel from outside the actor
            // unless we have the Sender.
            // For now, we'll just unload it, which is effectively a "retry soon".
            self.unload_cache(room_id).await;
        }
    }

    /// unload a single room from cache
    pub async fn unload_cache(&self, room_id: RoomId) {
        self.actors.invalidate(&room_id).await;
    }

    /// unload all rooms from cache
    pub fn unload_all_cache(&self) {
        self.actors.invalidate_all();
    }

    // TODO: make this not require writing room
    pub async fn get(&self, room_id: RoomId, user_id: Option<UserId>) -> Result<Room> {
        let snapshot = self.load_room(room_id, false).await?;
        let mut room = snapshot.get_data().unwrap().room.clone();

        if let Some(user_id) = user_id {
            let preferences = self
                .state
                .data()
                .preferences_room_get(user_id, room_id)
                .await?;
            room.preferences = Some(preferences);
        }

        Ok(room)
    }

    pub async fn invalidate(&self, room_id: RoomId) {
        self.unload_cache(room_id).await;
    }

    pub async fn reload(&self, room_id: RoomId) -> Result<()> {
        let room = self.state.data().room_get(room_id).await?;
        self.update_cache(room).await;
        Ok(())
    }

    /// register a user as being in a room in the cache
    pub fn member_register(&self, user_id: UserId, room_id: RoomId) {
        self.user_rooms
            .entry(user_id)
            .or_insert_with(DashSet::new)
            .insert(room_id);
    }

    /// unregister a user from a room in the cache
    pub fn member_unregister(&self, user_id: UserId, room_id: RoomId) {
        if let Some(rooms) = self.user_rooms.get(&user_id) {
            rooms.remove(&room_id);
            if rooms.is_empty() {
                drop(rooms);
                self.user_rooms.remove(&user_id);
            }
        }
    }

    pub fn purge_cache(&self) {
        self.unload_all_cache();
    }

    /// Try to send a sync message to a room actor. Returns Ok(true) if sent, Ok(false) if actor is dead.
    async fn try_send_sync(&self, room_id: RoomId, sync: MessageSync) -> Result<bool> {
        let handle = self.actors.get(&room_id).await;
        let Some(handle) = handle else {
            return Ok(false);
        };

        match handle.actor_ref.tell(SyncMessage { sync }).await {
            Ok(_) => Ok(true),
            Err(_) => {
                // Actor is dead, evict it so next request will respawn
                self.unload_cache(room_id).await;
                Ok(false)
            }
        }
    }

    /// update a room's metadata in the cache
    pub async fn update_cache(&self, room: Room) {
        let room_id = room.id;
        let _ = self
            .try_send_sync(room_id, MessageSync::RoomUpdate { room })
            .await;
    }

    /// reload a member from the database and update the cache
    pub async fn reload_member(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        let member = self.state.data().room_member_get(room_id, user_id).await?;
        let user = self.state.services().users.get(user_id, None).await?;
        let _ = self
            .try_send_sync(room_id, MessageSync::RoomMemberUpdate { member, user })
            .await;
        Ok(())
    }

    /// remove a member from the cache
    pub async fn remove_member(&self, room_id: RoomId, user_id: UserId) {
        let _ = self
            .try_send_sync(room_id, MessageSync::RoomMemberDelete { room_id, user_id })
            .await;
    }

    /// reload a role from the database and update the cache
    pub async fn reload_role(&self, room_id: RoomId, role_id: RoleId) -> Result<()> {
        let role = self.state.data().role_select(room_id, role_id).await?;
        let _ = self
            .try_send_sync(room_id, MessageSync::RoleUpdate { role })
            .await;
        Ok(())
    }

    /// remove a role from the cache
    pub async fn remove_role(&self, room_id: RoomId, role_id: RoleId) {
        let _ = self
            .try_send_sync(room_id, MessageSync::RoleDelete { room_id, role_id })
            .await;
    }

    /// reload a channel from the database and update the cache
    pub async fn reload_channel(&self, room_id: RoomId, channel_id: ChannelId) -> Result<()> {
        let channel = self.state.data().channel_get(channel_id).await?;
        let _ = self
            .try_send_sync(
                room_id,
                MessageSync::ChannelUpdate {
                    channel: Box::new(channel),
                },
            )
            .await;
        Ok(())
    }

    /// remove a channel from the cache
    pub async fn remove_channel(&self, room_id: RoomId, _channel_id: ChannelId) {
        // We don't have a direct "Delete" event that only takes ID for channel,
        // but we can send a dummy ChannelUpdate with is_removed = true if needed,
        // or just use unload_room if it's simpler.
        self.unload_cache(room_id).await;
    }

    /// reload a thread member from the database and update the cache
    pub async fn reload_thread_member(
        &self,
        room_id: RoomId,
        thread_id: ChannelId,
        user_id: UserId,
    ) -> Result<()> {
        let member = self
            .state
            .data()
            .thread_member_get(thread_id, user_id)
            .await?;
        let _ = self
            .try_send_sync(
                room_id,
                MessageSync::ThreadMemberUpsert {
                    room_id: Some(room_id),
                    thread_id,
                    added: vec![member],
                    removed: vec![],
                },
            )
            .await;
        Ok(())
    }

    /// remove a thread member from the cache
    pub async fn remove_thread_member(
        &self,
        room_id: RoomId,
        thread_id: ChannelId,
        user_id: UserId,
    ) {
        let _ = self
            .try_send_sync(
                room_id,
                MessageSync::ThreadMemberUpsert {
                    room_id: Some(room_id),
                    thread_id,
                    added: vec![],
                    removed: vec![user_id],
                },
            )
            .await;
    }

    pub async fn update(&self, room_id: RoomId, auth: Auth, patch: RoomPatch) -> Result<Room> {
        let al = auth.audit_log(room_id);
        let data = self.state.data();
        let srv = self.state.services();
        let user_id = auth.user.id;
        let start = data.room_get(room_id).await?;
        if !patch.changes(&start) {
            return Ok(start);
        }

        if let Some(icon) = &patch.icon {
            if start.icon.is_some() {
                data.media_link_delete_all(*room_id).await?;
            }
            if let Some(media_id) = icon {
                data.media_link_insert(*media_id, *room_id, MediaLinkType::RoomIcon)
                    .await?;
            }
        }

        if let Some(banner) = &patch.banner {
            if start.banner.is_some() {
                data.media_link_delete_all(*room_id).await?;
            }
            if let Some(media_id) = banner {
                data.media_link_insert(*media_id, *room_id, MediaLinkType::RoomBanner)
                    .await?;
            }
        }

        if let Some(Some(chan_id)) = patch.welcome_channel_id {
            let chan = srv.channels.get(chan_id, None).await?;
            if chan.ty != ChannelType::Text {
                return Err(Error::BadStatic("welcome channel must be text"));
            }
        }

        data.room_update(room_id, patch).await?;
        data.room_template_mark_dirty(room_id).await?;

        let updated_room = data.room_get(room_id).await?;
        self.state
            .services()
            .rooms
            .update_cache(updated_room.clone())
            .await;

        let mut end = updated_room;
        if let Some(user_id) = Some(user_id) {
            let preferences = self
                .state
                .data()
                .preferences_room_get(user_id, room_id)
                .await?;
            end.preferences = Some(preferences);
        }

        let snapshot = self.load_room(room_id, false).await?;
        let data = snapshot.get_data().unwrap();
        end.online_count = data.room.online_count;
        end.member_count = data.room.member_count;

        let changes = Changes::new()
            .change("name", &start.name, &end.name)
            .change("description", &start.description, &end.description)
            .change("icon", &start.icon, &end.icon)
            .change("banner", &start.banner, &end.banner)
            .change("public", &start.public, &end.public)
            .change(
                "welcome_channel_id",
                &start.welcome_channel_id,
                &end.welcome_channel_id,
            )
            .change("afk_channel_id", &start.afk_channel_id, &end.afk_channel_id)
            .change(
                "afk_channel_timeout",
                &start.afk_channel_timeout,
                &end.afk_channel_timeout,
            )
            .build();

        al.commit(
            AuditLogEntryStatus::Success,
            AuditLogEntryType::RoomUpdate { changes },
        )
        .await?;

        self.state
            .broadcast_room(
                room_id,
                user_id,
                MessageSync::RoomUpdate { room: end.clone() },
            )
            .await?;

        Ok(end)
    }

    pub async fn create(
        &self,
        create: RoomCreate,
        auth: &Auth,
        extra: DbRoomCreate,
        nonce: Option<String>,
    ) -> Result<Room> {
        if let Some(n) = &nonce {
            self.idempotency_keys
                .try_get_with(
                    n.clone(),
                    self.create_inner(create, auth.user.id, Some(auth), extra, nonce.clone()),
                )
                .await
                .map_err(|err| err.fake_clone())
        } else {
            self.create_inner(create, auth.user.id, Some(auth), extra, nonce)
                .await
        }
    }

    pub async fn create_system(
        &self,
        create: RoomCreate,
        user_id: UserId,
        extra: DbRoomCreate,
    ) -> Result<Room> {
        self.create_inner(create, user_id, None, extra, None).await
    }

    async fn create_inner(
        &self,
        create: RoomCreate,
        creator_id: UserId,
        auth: Option<&Auth>,
        extra: DbRoomCreate,
        nonce: Option<String>,
    ) -> Result<Room> {
        create.validate()?;
        let data = self.state.data();
        let srv = self.state.services();
        let welcome_channel_id = extra.welcome_channel_id;
        let mut room = data.room_create(create.clone(), extra).await?;
        let room_id = room.id;

        data.room_member_put(
            room_id,
            creator_id,
            Some(RoomMemberOrigin::Creator),
            RoomMemberPut::default(),
        )
        .await?;
        data.room_set_owner(room_id, creator_id).await?;
        room.owner_id = Some(creator_id);

        self.state
            .services()
            .perms
            .invalidate_room(creator_id, room_id)
            .await;

        let mut template_items = None;

        if welcome_channel_id.is_none() {
            let snapshot = if create.public.unwrap_or_default() {
                builtin::public_room()
            } else {
                builtin::private_room()
            };

            template_items = Some(
                srv.room_templates
                    .apply_to_room(room_id, creator_id, snapshot)
                    .await?,
            );
        }

        // reload room to get updated welcome_channel_id and other stuff set by apply_to_room
        let mut room = data.room_get(room_id).await?;
        room.owner_id = Some(creator_id);

        self.state.broadcast_with_nonce(
            nonce.as_deref(),
            MessageSync::RoomCreate { room: room.clone() },
        )?;

        if let Some((roles, channels)) = template_items {
            for role in roles {
                self.state
                    .broadcast_room(room_id, creator_id, MessageSync::RoleCreate { role })
                    .await?;
            }

            for channel in channels {
                self.state
                    .broadcast_room(
                        room_id,
                        creator_id,
                        MessageSync::ChannelCreate {
                            channel: Box::new(channel),
                        },
                    )
                    .await?;
            }
        }

        if let Some(auth) = auth {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::RoomCreate {
                changes: Changes::new()
                    .add("name", &room.name)
                    .add("description", &room.description)
                    .add("icon", &room.icon)
                    .add("banner", &room.banner)
                    .add("public", &room.public)
                    .add("welcome_channel_id", &room.welcome_channel_id)
                    .build(),
            })
            .await?;
        }

        if room.welcome_channel_id.is_some() {
            self.send_welcome_message(room_id, creator_id).await?;
        }

        Ok(room)
    }

    /// sends a MemberJoin message in the default/welcome thread
    pub async fn send_welcome_message(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        let room = self.get(room_id, None).await?;

        if let Some(wti) = room.welcome_channel_id {
            let data = self.state.data();
            let welcome_message_id = data
                .message_create(DbMessageCreate {
                    id: None,
                    channel_id: wti,
                    attachment_ids: vec![],
                    author_id: user_id,
                    embeds: vec![],
                    message_type: MessageType::MemberJoin.into(),
                    created_at: None,
                    removed_at: None,
                    mentions: Default::default(),
                })
                .await?;
            let welcome_message = data.message_get(wti, welcome_message_id, user_id).await?;

            self.state
                .broadcast_channel(
                    wti,
                    user_id,
                    MessageSync::MessageCreate {
                        message: welcome_message,
                    },
                )
                .await?;

            let tm = data.thread_member_get(wti, user_id).await;
            if tm.is_err() {
                data.thread_member_put(wti, user_id, ThreadMemberPut::default())
                    .await?;
                let thread_member = data.thread_member_get(wti, user_id).await?;
                let msg = MessageSync::ThreadMemberUpsert {
                    room_id: Some(room_id),
                    thread_id: wti,
                    added: vec![thread_member],
                    removed: vec![],
                };
                self.state.broadcast_channel(wti, user_id, msg).await?;
            }
        }

        Ok(())
    }

    /// add private user data to each room
    pub async fn populate_private(&self, rooms: &mut [Room], user_id: UserId) -> Result<()> {
        if rooms.is_empty() {
            return Ok(());
        }

        let data = self.state.data();

        // collect all room ids for batch fetching
        let room_ids: Vec<_> = rooms.iter().map(|r| r.id).collect();

        // fetch preferences for all rooms
        let preferences_map = data.preferences_room_get_many(user_id, &room_ids).await?;

        // populate each room with private data
        for room in rooms {
            if let Some(config) = preferences_map.get(&room.id) {
                room.preferences = Some(config.clone());
            }
        }

        Ok(())
    }
}
