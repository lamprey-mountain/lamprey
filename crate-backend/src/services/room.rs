use std::sync::Arc;

use common::v1::types::defaults::{EVERYONE_TRUSTED, MODERATOR};
use common::v1::types::presence::Status;
use common::v1::types::util::{Changes, Diff};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryStatus, AuditLogEntryType, ChannelType,
    MessageSync, MessageType, Permission, RoleId, Room, RoomCreate, RoomId, RoomMemberOrigin,
    RoomMemberPut, RoomPatch, ThreadMemberPut, ThreadMembership, UserId,
};

use crate::error::Result;
use crate::routes::util::Auth;
use crate::types::{
    DbChannelCreate, DbChannelType, DbMessageCreate, DbRoleCreate, DbRoomCreate, MediaLinkType,
};
use crate::{Error, ServerStateInner};

pub struct ServiceRooms {
    state: Arc<ServerStateInner>,
}

impl ServiceRooms {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { state }
    }

    // TODO: make this not require writing room
    pub async fn get(&self, room_id: RoomId, user_id: Option<UserId>) -> Result<Room> {
        let srv = self.state.services();
        let cached = srv.cache.load_room(room_id).await?;
        let mut room = cached.inner.write().await;

        if let Some(user_id) = user_id {
            let user_config = self
                .state
                .data()
                .user_config_room_get(user_id, room_id)
                .await?;
            room.user_config = Some(user_config);
        }

        let mut online_count = 0;
        for member in &cached.members {
            if srv.presence.get(*member.key()).status != Status::Offline {
                online_count += 1;
            }
        }
        room.online_count = online_count;
        room.member_count = cached.members.len() as u64;

        Ok(room.to_owned())
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

        if let Some(Some(chan_id)) = patch.welcome_channel_id {
            let chan = srv.channels.get(chan_id, None).await?;
            if chan.ty != ChannelType::Text {
                return Err(Error::BadStatic("welcome channel must be text"));
            }
        }

        data.room_update(room_id, patch).await?;

        let updated_room = data.room_get(room_id).await?;
        self.state.services().cache.update_room(updated_room).await;
        let end = self.get(room_id, Some(user_id)).await?;

        let changes = Changes::new()
            .change("name", &start.name, &end.name)
            .change("description", &start.description, &end.description)
            .change("icon", &start.icon, &end.icon)
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

        Ok(end)
    }

    pub async fn create(
        &self,
        create: RoomCreate,
        creator_id: UserId,
        extra: DbRoomCreate,
    ) -> Result<Room> {
        let data = self.state.data();
        let welcome_channel_id = extra.welcome_channel_id;
        let mut room = data.room_create(create, extra).await?;
        let room_id = room.id;

        let role_admin = DbRoleCreate {
            id: RoleId::new(),
            room_id,
            name: "admin".to_owned(),
            description: None,
            allow: vec![Permission::Admin],
            deny: vec![],
            is_self_applicable: false,
            is_mentionable: false,
            hoist: false,
        };
        let role_moderator = DbRoleCreate {
            id: RoleId::new(),
            room_id,
            name: "moderator".to_owned(),
            description: None,
            allow: MODERATOR.to_vec(),
            deny: vec![],
            is_self_applicable: false,
            is_mentionable: false,
            hoist: false,
        };
        let role_everyone = DbRoleCreate {
            id: RoleId::from(room.id.into_inner()),
            room_id,
            name: "everyone".to_owned(),
            description: Some("Default role".to_string()),
            allow: EVERYONE_TRUSTED.to_vec(),
            deny: vec![],
            is_self_applicable: false,
            is_mentionable: false,
            hoist: false,
        };
        data.role_create(role_admin, 1).await?;
        data.role_create(role_moderator, 1).await?;
        data.role_create(role_everyone, 0).await?;
        data.room_member_put(
            room_id,
            creator_id,
            Some(RoomMemberOrigin::Creator),
            RoomMemberPut::default(),
        )
        .await?;
        data.room_set_owner(room_id, creator_id).await?;
        room.owner_id = Some(creator_id);

        let (welcome_channel_id, welcome_channel) = if let Some(channel_id) = welcome_channel_id {
            (channel_id, None)
        } else {
            let welcome_channel_id = data
                .channel_create(DbChannelCreate {
                    room_id: Some(room.id.into_inner()),
                    creator_id,
                    name: "general".to_string(),
                    description: None,
                    url: None,
                    ty: DbChannelType::Text,
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
            let welcome_channel = data.channel_get(welcome_channel_id).await?;
            (welcome_channel_id, Some(welcome_channel))
        };

        data.room_update(
            room_id,
            RoomPatch {
                welcome_channel_id: Some(Some(welcome_channel_id)),
                name: None,
                description: None,
                icon: None,
                public: None,
                afk_channel_id: None,
                afk_channel_timeout: None,
            },
        )
        .await?;
        room.welcome_channel_id = Some(welcome_channel_id);

        self.state
            .broadcast(MessageSync::RoomCreate { room: room.clone() })?;

        self.state
            .audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: creator_id,
                session_id: None, // TODO: get session id
                reason: None,     // TODO: get reason
                ty: AuditLogEntryType::RoomCreate {
                    changes: Changes::new()
                        .add("name", &room.name)
                        .add("description", &room.description)
                        .add("icon", &room.icon)
                        .add("public", &room.public)
                        .add("welcome_channel_id", &room.welcome_channel_id)
                        .build(),
                },
            })
            .await?;

        if let Some(welcome_thread) = welcome_channel {
            self.state
                .broadcast_room(
                    room_id,
                    creator_id,
                    MessageSync::ChannelCreate {
                        channel: Box::new(welcome_thread),
                    },
                )
                .await?;

            self.state
                .audit_log_append(AuditLogEntry {
                    id: AuditLogEntryId::new(),
                    room_id,
                    user_id: creator_id,
                    session_id: None, // TODO: get session id
                    reason: None,     // TODO: get reason
                    ty: AuditLogEntryType::ChannelCreate {
                        channel_id: welcome_channel_id,
                        channel_type: ChannelType::Text,
                        changes: Changes::new()
                            .add("name", &"general")
                            .add("nsfw", &false)
                            .build(),
                    },
                })
                .await?;

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
                    message_type: MessageType::MemberJoin,
                    edited_at: None,
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
            if tm.is_err() || tm.is_ok_and(|tm| tm.membership == ThreadMembership::Leave) {
                data.thread_member_put(wti, user_id, ThreadMemberPut::default())
                    .await?;
                let thread_member = data.thread_member_get(wti, user_id).await?;
                let msg = MessageSync::ThreadMemberUpsert {
                    member: thread_member,
                };
                self.state.broadcast_channel(wti, user_id, msg).await?;
            }
        }

        Ok(())
    }

    /// add private user data to each room
    pub async fn merge(&self, rooms: &mut [Room], user_id: UserId) -> Result<()> {
        Ok(())
    }
}
