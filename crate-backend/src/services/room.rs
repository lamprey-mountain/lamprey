use std::sync::Arc;
use std::time::Duration;

use common::v1::types::util::{Changes, Diff};
use common::v1::types::{
    AuditLogEntryStatus, AuditLogEntryType, ChannelType, MessageSync, MessageType, Room,
    RoomCreate, RoomId, RoomMemberOrigin, RoomMemberPut, RoomPatch, ThreadMemberPut, UserId,
};
use moka::future::Cache;
use validator::Validate;

use crate::error::Result;
use crate::routes::util::Auth;
use crate::services::room_template::builtin;
use crate::types::{DbMessageCreate, DbRoomCreate, MediaLinkType};
use crate::{Error, ServerStateInner};

pub struct ServiceRooms {
    state: Arc<ServerStateInner>,
    idempotency_keys: Cache<String, Room>,
}

impl ServiceRooms {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            idempotency_keys: Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    // TODO: make this not require writing room
    pub async fn get(&self, room_id: RoomId, user_id: Option<UserId>) -> Result<Room> {
        let srv = self.state.services();
        let snapshot = srv.cache.load_room(room_id).await?;
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
        self.state.services().cache.unload_room(room_id).await;
    }

    pub async fn reload(&self, room_id: RoomId) -> Result<()> {
        let room = self.state.data().room_get(room_id).await?;
        self.state.services().cache.update_room(room).await;
        Ok(())
    }

    pub fn purge_cache(&self) {
        self.state.services().cache.unload_all();
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
            .cache
            .update_room(updated_room.clone())
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

        let snapshot = self.state.services().cache.load_room(room_id).await?;
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
