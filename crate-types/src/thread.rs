use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::{MessageId, RoomId, ThreadId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Thread {
    // pub thread_type: ThreadType,
    pub id: ThreadId,
    pub room_id: RoomId,
    pub creator_id: UserId,
    // FIXME: verify max and min lengths
    #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 64))]
    pub name: String,
    #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 2048))]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadCreateRequest {
    #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 64))]
    pub name: String,
    #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 2048))]
    pub description: Option<String>,
    pub is_closed: Option<bool>,
    pub is_locked: Option<bool>,
    pub is_pinned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadPatch {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub is_closed: Option<bool>,
    pub is_locked: Option<bool>,
    pub is_pinned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum ThreadInfo {
    Foo { a: u64 },
    Bar { b: bool },
}

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// #[sqlx(type_name = "thread_type")]
// pub enum ThreadType {
// 	Chat,
// }

// pub enum ThreadState {
//     /// always remains active
//     Pinned,

//     /// default state that new threads are in
//     Active,

//     /// goes straight to Deleted instead of Archived
//     Temporary,

//     /// inactive
//     Archived,

//     /// will be permanently deleted soon, visible to moderators
//     Deleted,
// }
