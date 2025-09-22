use std::sync::Arc;

use common::v1::types::defaults::{EVERYONE_TRUSTED, MODERATOR};
use common::v1::types::util::{Changes, Diff};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, MessageType, Permission,
    RoleId, Room, RoomCreate, RoomId, RoomMemberOrigin, RoomMemberPut, RoomPatch, UserId,
};
use moka::future::Cache;

use crate::error::{Error, Result};
use crate::types::{
    DbMessageCreate, DbRoleCreate, DbRoomCreate, DbThreadCreate, DbThreadType, MediaLinkType,
};
use crate::ServerStateInner;

pub struct ServiceRooms {
    state: Arc<ServerStateInner>,
    cache_room: Cache<RoomId, Room>,
}

impl ServiceRooms {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            cache_room: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
        }
    }

    pub async fn get(&self, room_id: RoomId, _user_id: Option<UserId>) -> Result<Room> {
        self.cache_room
            .try_get_with(room_id, self.state.data().room_get(room_id))
            .await
            .map_err(|err| err.fake_clone())
    }

    pub async fn invalidate(&self, room_id: RoomId) {
        self.cache_room.invalidate(&room_id).await;
    }

    pub async fn update(
        &self,
        room_id: RoomId,
        user_id: UserId,
        patch: RoomPatch,
        reason: Option<String>,
    ) -> Result<Room> {
        let data = self.state.data();
        let start = data.room_get(room_id).await?;
        if !patch.changes(&start) {
            return Err(Error::NotModified);
        }

        if let Some(icon) = &patch.icon {
            if start.icon.is_some() {
                data.media_link_delete_all(*room_id).await?;
            }
            if let Some(media_id) = icon {
                data.media_link_insert(*media_id, *room_id, MediaLinkType::AvatarRoom)
                    .await?;
            }
        }

        data.room_update(room_id, patch).await?;

        self.cache_room.invalidate(&room_id).await;
        let end = self.get(room_id, Some(user_id)).await?;

        let changes = Changes::new()
            .change("name", &start.name, &end.name)
            .change("description", &start.description, &end.description)
            .change("icon", &start.icon, &end.icon)
            .change("public", &start.public, &end.public)
            .change(
                "welcome_thread_id",
                &start.welcome_thread_id,
                &end.welcome_thread_id,
            )
            .build();

        self.state
            .audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id,
                session_id: None, // TODO: get session id
                reason,
                ty: AuditLogEntryType::RoomUpdate { changes },
            })
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
        let mut room = data.room_create(create, extra).await?;
        let room_id = room.id;

        let role_admin = DbRoleCreate {
            id: RoleId::new(),
            room_id,
            name: "admin".to_owned(),
            description: None,
            permissions: vec![Permission::Admin],
            is_self_applicable: false,
            is_mentionable: false,
        };
        let role_moderator = DbRoleCreate {
            id: RoleId::new(),
            room_id,
            name: "moderator".to_owned(),
            description: None,
            permissions: MODERATOR.to_vec(),
            is_self_applicable: false,
            is_mentionable: false,
        };
        let role_everyone = DbRoleCreate {
            id: RoleId::from(room.id.into_inner()),
            room_id,
            name: "everyone".to_owned(),
            description: Some("Default role".to_string()),
            permissions: EVERYONE_TRUSTED.to_vec(),
            is_self_applicable: false,
            is_mentionable: false,
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

        let welcome_thread_id = data
            .thread_create(DbThreadCreate {
                room_id: Some(room.id.into_inner()),
                creator_id,
                name: "general".to_string(),
                description: None,
                ty: DbThreadType::Chat,
                nsfw: false,
            })
            .await?;
        let welcome_thread = data.thread_get(welcome_thread_id).await?;

        data.room_update(
            room_id,
            RoomPatch {
                welcome_thread_id: Some(Some(welcome_thread_id)),
                name: None,
                description: None,
                icon: None,
                public: None,
            },
        )
        .await?;
        room.welcome_thread_id = Some(welcome_thread_id);

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
                        .add("welcome_thread_id", &room.welcome_thread_id)
                        .build(),
                },
            })
            .await?;

        self.state
            .broadcast_room(
                room_id,
                creator_id,
                MessageSync::ThreadCreate {
                    thread: welcome_thread,
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
                ty: AuditLogEntryType::ThreadCreate {
                    thread_id: welcome_thread_id,
                    changes: Changes::new()
                        .add("name", &"general")
                        .add("nsfw", &false)
                        .build(),
                },
            })
            .await?;

        self.send_welcome_message(room_id, creator_id).await?;

        Ok(room)
    }

    /// sends a MemberJoin message in the default/welcome thread
    pub async fn send_welcome_message(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        let room = self.get(room_id, None).await?;

        if let Some(wti) = room.welcome_thread_id {
            let data = self.state.data();
            let welcome_message_id = data
                .message_create(DbMessageCreate {
                    thread_id: wti,
                    attachment_ids: vec![],
                    author_id: user_id,
                    embeds: vec![],
                    message_type: MessageType::MemberJoin,
                    edited_at: None,
                    created_at: None,
                })
                .await?;
            let welcome_message = data.message_get(wti, welcome_message_id, user_id).await?;

            self.state
                .broadcast_thread(
                    wti,
                    user_id,
                    MessageSync::MessageCreate {
                        message: welcome_message,
                    },
                )
                .await?;
        }

        Ok(())
    }
}
