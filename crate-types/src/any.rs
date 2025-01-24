use crate::{Invite, Media, Message, Role, Room, Thread, User};

use serde::{Deserialize, Serialize};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
/// might be useful in some places, but "more restrictive types" are still probably better
pub enum Anything {
    User {
        user: User,
    },

    Room {
        room: Room,
    },

    Thread {
        room: Room,
        thread: Thread,
    },

    Message {
        room: Room,
        thread: Thread,
        message: Message,
    },

    Role {
        room: Room,
        role: Role,
    },

    Invite {
        invite: Invite,
    },

    Media {
        media: Media,
    },
}
