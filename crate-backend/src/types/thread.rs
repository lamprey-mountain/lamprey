use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{MessageId, RoomId, ThreadId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Thread {
    // pub thread_type: ThreadType,
    pub id: ThreadId,
    pub room_id: RoomId,
    pub creator_id: UserId,
    // FIXME: verify max and min lengths
    #[schema(max_length = 1, min_length = 64)]
    pub name: String,
    #[schema(max_length = 1, min_length = 2048)]
    pub description: Option<String>,
    pub is_closed: bool,
    pub is_locked: bool,
    pub is_pinned: bool,
    // is_wiki: z.boolean(), // editable by everyone
    // is_private: z.boolean(),
    // recipients: Member.array(),
    // #[serde(flatten)]
    // info: ThreadInfo,
    // TODO: split out is_unread to be able to filter out blocked users server side?
    pub is_unread: bool,
    pub last_version_id: MessageId,
    pub last_read_id: Option<MessageId>,
    pub message_count: u64,
    // mention_count: z.number(),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadRow {
    pub id: ThreadId,
    pub room_id: RoomId,
    pub creator_id: UserId,
    pub name: String,
    pub description: Option<String>,
    pub is_closed: bool,
    pub is_locked: bool,
    pub is_pinned: bool,
    pub is_unread: bool,
    pub last_version_id: MessageId,
    pub last_read_id: Option<Uuid>,
    pub message_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct ThreadCreateRequest {
    #[schema(max_length = 1, min_length = 64)]
    pub name: String,
    #[schema(max_length = 1, min_length = 2048)]
    pub description: Option<String>,
    pub is_closed: Option<bool>,
    pub is_locked: Option<bool>,
    pub is_pinned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadCreate {
    pub room_id: RoomId,
    pub creator_id: UserId,
    pub name: String,
    pub description: Option<String>,
    pub is_closed: bool,
    pub is_locked: bool,
    pub is_pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct ThreadPatch {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub is_closed: Option<bool>,
    pub is_locked: Option<bool>,
    pub is_pinned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ThreadInfo {
    Foo { a: u64 },
    Bar { b: bool },
}

// #[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
// #[sqlx(type_name = "thread_type")]
// pub enum ThreadType {
// 	Default,
// }

impl From<ThreadRow> for Thread {
    fn from(row: ThreadRow) -> Self {
        Thread {
            id: row.id,
            room_id: row.room_id,
            creator_id: row.creator_id,
            name: row.name,
            description: row.description,
            is_closed: row.is_closed,
            is_locked: row.is_locked,
            is_pinned: row.is_pinned,
            is_unread: row.is_unread,
            last_version_id: row.last_version_id,
            last_read_id: row.last_read_id.map(Into::into),
            message_count: row.message_count.try_into().expect("count is negative?"),
        }
    }
}
