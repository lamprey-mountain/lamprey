use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{util::Diff, ThreadVerId};

use super::{MessageId, RoomId, ThreadId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Thread {
    pub id: ThreadId,
    pub room_id: RoomId,
    pub creator_id: UserId,
    pub version_id: ThreadVerId,

    #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 64))]
    pub name: String,

    #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 2048))]
    pub description: Option<String>,

    pub state: ThreadState,
    pub visibility: ThreadVisibility,

    #[serde(flatten)]
    pub info: ThreadInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadCreateRequest {
    #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 64))]
    pub name: String,
    #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 2048))]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadPatch {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub state: Option<ThreadState>,
}

/// lifecycle of a thread
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ThreadState {
    /// always remains active
    Pinned { pin_order: u32 },

    /// default state that new threads are in
    Active,

    /// goes straight to Deleted instead of Archived
    Temporary,

    /// inactive
    Archived,

    /// will be permanently deleted soon, visible to moderators
    Deleted,
}

/// who can view this thread
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ThreadVisibility {
    /// Everyone in the room can view
    Room,
    // Public
    // Private { recipients: Vec<UserId> }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum ThreadInfo {
    Chat {
        is_unread: bool,
        last_version_id: MessageId,
        last_read_id: Option<MessageId>,
        message_count: u64,
    },
}

impl Diff<Thread> for ThreadPatch {
    fn changes(&self, other: &Thread) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.state.changes(&other.state)
    }
}

impl Diff<Thread> for ThreadState {
    fn changes(&self, other: &Thread) -> bool {
        self != &other.state
    }
}

impl ThreadState {
    pub fn can_change_to(&self, _to: &ThreadState) -> bool {
        match self {
            Self::Deleted => false,
            _ => true,
        }
    }
}
