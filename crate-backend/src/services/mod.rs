use std::sync::Arc;

use media::ServiceMedia;
use serde_json::json;
use types::util::Diff;
use types::{
    MessageSync, MessageType, Permission, Room, RoomCreate, RoomMembership, Thread, ThreadId,
    ThreadPatch, UserId,
};

use crate::error::{Error, Result};
use crate::types::MessageCreate;
use crate::ServerState;
use crate::{data::Data, types::RoleCreate};

pub mod media;
pub mod oauth2;

pub struct Services {
    state: Arc<ServerState>,
    data: Box<dyn Data>,
    pub media: ServiceMedia,
}

impl Services {
    pub fn new(state: Arc<ServerState>, data: Box<dyn Data>) -> Self {
        Self {
            state,
            data,
            media: ServiceMedia::new(),
        }
    }

    pub async fn create_room(&self, create: RoomCreate, creator: UserId) -> Result<Room> {
        let room = self.data.room_create(create).await?;
        let room_id = room.id;
        let role_admin = RoleCreate {
            room_id,
            name: "admin".to_owned(),
            description: None,
            permissions: vec![Permission::Admin],
            is_self_applicable: false,
            is_mentionable: false,
            is_default: false,
        };
        let role_moderator = RoleCreate {
            room_id,
            name: "moderator".to_owned(),
            description: None,
            permissions: vec![
                Permission::ThreadManage,
                Permission::ThreadDelete,
                Permission::MessagePin,
                Permission::MessageDelete,
                Permission::MemberKick,
                Permission::MemberBan,
                Permission::MemberManage,
                Permission::InviteManage,
            ],
            is_self_applicable: false,
            is_mentionable: false,
            is_default: false,
        };
        let role_everyone = RoleCreate {
            room_id,
            name: "everyone".to_owned(),
            description: None,
            permissions: vec![
                Permission::MessageCreate,
                Permission::MessageFilesEmbeds,
                Permission::ThreadCreate,
                Permission::InviteCreate,
            ],
            is_self_applicable: false,
            is_mentionable: false,
            is_default: true,
        };
        let admin = self.data.role_create(role_admin).await?;
        self.data.role_create(role_moderator).await?;
        self.data.role_create(role_everyone).await?;
        self.data
            .room_member_put(
                room_id,
                creator,
                RoomMembership::Join {
                    override_name: None,
                    override_description: None,
                    roles: vec![],
                },
            )
            .await?;
        self.data.role_member_put(creator, admin.id).await?;
        self.data.role_apply_default(room.id, creator).await?;
        Ok(room)
    }

    pub async fn update_thread(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        patch: ThreadPatch,
    ) -> Result<Thread> {
        // check update perms
        let mut perms = self.data.permission_thread_get(user_id, thread_id).await?;
        perms.ensure_view()?;
        let thread = self.data.thread_get(thread_id, Some(user_id)).await?;
        if thread.creator_id == user_id {
            perms.add(Permission::ThreadManage);
        }
        perms.ensure(Permission::ThreadManage)?;

        // shortcut if it wont modify the thread
        if !patch.changes(&thread) {
            return Err(Error::NotModified);
        }

        if let Some(new_state) = &patch.state {
            if !thread.state.can_change_to(&new_state) {
                return Err(Error::BadStatic("can't change to that state"));
            }
        };

        // update and refetch
        self.data
            .thread_update(thread_id, user_id, patch.clone())
            .await?;
        let thread = self.data.thread_get(thread_id, Some(user_id)).await?;

        // send update message to thread
        let update_message_id = self
            .data
            .message_create(MessageCreate {
                thread_id,
                content: Some("(thread update)".to_string()),
                attachment_ids: vec![],
                author_id: user_id,
                message_type: MessageType::ThreadUpdate,
                metadata: Some(json!({
                    "name": patch.name,
                    "description": patch.description,
                })),
                reply_id: None,
                override_name: None,
            })
            .await?;
        let update_message = self.data.message_get(thread_id, update_message_id).await?;

        self.state.broadcast(MessageSync::UpsertMessage {
            message: update_message,
        })?;
        let msg = MessageSync::UpsertThread {
            thread: thread.clone(),
        };
        self.state
            .broadcast_room(thread.room_id, user_id, None, msg)
            .await?;

        Ok(thread)
    }
}
