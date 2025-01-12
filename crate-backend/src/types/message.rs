use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid7::Uuid;

use super::{Media, MessageId, ThreadId, User};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(examples("00000000-0000-0000-0000-00messagever"))]
pub struct MessageVersionId(Uuid);

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Message {
    id: MessageId,
    thread_id: ThreadId,
    version_id: MessageVersionId,
    nonce: Option<String>,
    ordering: u64,
    content: Option<String>,
    attachments: Vec<Media>,
    // embeds: Embed.array().default([]),
    // metadata: z.record(z.string(), z.any()).nullable(),
    // mentions_users: UserId.array(),
    // mentions_roles: RoleId.array(),
    // mentions_everyone: z.boolean(),
    // reply_id: MessageId.nullable(),
    // resolve everything here?
    // mentions_threads: ThreadId.array(),
    // mentions_rooms: ThreadId.array(),
    // author: Member, // TODO: future? how to represent users who have left?
    author: User,
    is_pinned: bool,
}
