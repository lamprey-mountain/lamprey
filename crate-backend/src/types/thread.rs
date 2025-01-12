use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{RoomId, ThreadId, UserId};

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Thread {
    id: ThreadId,
    room_id: RoomId,
    creator_id: UserId,
    #[schema(max_length = 1, min_length = 64)]
    name: String,
    #[schema(max_length = 1, min_length = 2048)]
    description: Option<String>,
    is_closed: bool,
    is_locked: bool,
    is_pinned: bool,
    // is_wiki: z.boolean(), // editable by everyone
    // is_private: z.boolean(),
    // recipients: Member.array(),
    // #[serde(flatten)]
    // info: ThreadInfo,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct ThreadPatch {
    name: Option<String>,
    description: Option<Option<String>>,
    is_closed: Option<bool>,
    is_locked: Option<bool>,
    is_pinned: Option<bool>,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
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
