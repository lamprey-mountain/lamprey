use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{
    user_status::StatusPatch, util::Time, InviteTargetId, InviteWithMetadata, Relationship,
    ThreadMember,
};

use super::{
    reaction::ReactionKey, InviteCode, Message, MessageId, MessageVerId, Role, RoleId, Room,
    RoomId, RoomMember, Session, SessionId, SessionToken, Thread, ThreadId, User, UserId,
};

mod sync2;

pub use sync2::{SyncCompression, SyncFormat, SyncParams, SyncVersion};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MessageClient {
    /// initial message
    Hello {
        token: SessionToken,

        status: Option<StatusPatch>,

        #[serde(flatten)]
        resume: Option<SyncResume>,
    },

    /// set status
    Status { status: StatusPatch },

    /// heartbeat
    Pong,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncResume {
    pub conn: String,
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageEnvelope {
    #[serde(flatten)]
    pub payload: MessagePayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "op")]
#[allow(clippy::large_enum_variant)]
pub enum MessagePayload {
    /// heartbeat
    Ping,

    /// data to keep local copy of state in sync with server
    Sync { data: MessageSync, seq: u64 },

    /// some kind of error
    Error { error: String },

    /// successfully connected
    Ready {
        /// current user, null if session is unauthed
        user: Option<User>,

        /// current session
        session: Session,

        /// connection id
        conn: String,

        /// sequence id for reconnecting
        seq: u64,
    },

    /// successfully reconnected
    Resumed,

    /// client needs to disconnect and reconnect
    Reconnect { can_resume: bool },
}

// TODO(#259): rename to NounVerb
// maybe replace *Delete with *Upsert with state = deleted (but don't send actual full item content)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
#[allow(clippy::large_enum_variant)]
pub enum MessageSync {
    UpsertRoom {
        room: Room,
    },
    UpsertThread {
        thread: Thread,
    },
    UpsertMessage {
        message: Message,
    },
    UpsertUser {
        user: User,
    },
    UpsertRoomMember {
        member: RoomMember,
    },
    UpsertThreadMember {
        member: ThreadMember,
    },
    UpsertSession {
        session: Session,
    },
    UpsertRole {
        role: Role,
    },
    UpsertInvite {
        invite: InviteWithMetadata,
    },
    DeleteMessage {
        /// deprecated = "keyed by thread_id"
        #[cfg_attr(feature = "utoipa", schema(deprecated))]
        room_id: RoomId,
        thread_id: ThreadId,
        message_id: MessageId,
    },
    DeleteMessageVersion {
        /// deprecated = "keyed by thread_id"
        #[cfg_attr(feature = "utoipa", schema(deprecated))]
        room_id: RoomId,
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    },
    DeleteUser {
        id: UserId,
    },
    DeleteSession {
        id: SessionId,
        user_id: Option<UserId>,
    },
    DeleteRole {
        room_id: RoomId,
        role_id: RoleId,
    },
    DeleteInvite {
        code: InviteCode,
        target: InviteTargetId,
    },

    Typing {
        thread_id: ThreadId,
        user_id: UserId,
        until: Time,
    },

    /// read receipt update
    ThreadAck {
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    },

    RelationshipUpsert {
        user_id: UserId,
        relationship: Relationship,
    },

    RelationshipDelete {
        user_id: UserId,
    },

    ReactionMessageUpsert {
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    },

    ReactionMessageRemove {
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    },

    /// remove all reactions
    ReactionMessagePurge {
        thread_id: ThreadId,
        message_id: MessageId,
    },

    ReactionThreadUpsert {
        user_id: UserId,
        thread_id: ThreadId,
        key: ReactionKey,
    },

    ReactionThreadRemove {
        user_id: UserId,
        thread_id: ThreadId,
        key: ReactionKey,
    },

    /// remove all reactions
    ReactionThreadPurge {
        thread_id: ThreadId,
    },
    // snip... ----- >8 ----
    // everything below is TODO

    // /// remove multiple messages at once
    // MessageDeleteBulk {
    //     thread_id: ThreadId,
    //     message_ids: Vec<MessageId>,
    // },

    // VoiceUpsert {
    //     thread_id: ThreadId,
    //     user_id: UserId,
    //     voice_state: VoiceState,
    // },

    // VoiceDelete {
    //     thread_id: ThreadId,
    //     user_id: UserId,
    // },

    // EventRsvp {
    //     thread_id: ThreadId,
    //     user_id: UserId,
    //     status: EventRsvpType,
    // },

    // DocumentEdit {
    //     thread_id: ThreadId,
    //     edit: DocumentEdit,
    // },

    // TableRowUpsert {
    //     thread_id: ThreadId,
    //     row: Row,
    // },

    // TableRowDelete {
    //     thread_id: ThreadId,
    //     row_id: RowId,
    // },

    // ReportUpsert {
    //     report: Report,
    // },

    // /// arbitrary user defined event
    // Dispatch {
    //     user_id: UserId,
    //     action: String,
    //     payload: Option<serde_json::Value>,
    // },
}

// TODO: maybe split out different message types

// /// messages specific to a user
// #[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
// #[serde(tag = "type")]
// enum MessageUser {}

// /// messages specific to a thread
// #[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
// #[serde(tag = "type")]
// enum MessageThread {}

// /// messages specific to a room
// #[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
// #[serde(tag = "type")]
// enum MessageRoom {}

impl MessageSync {
    pub fn is_room_audit_loggable(&self) -> bool {
        matches!(
            self,
            MessageSync::UpsertRoom { .. }
                | MessageSync::UpsertThread { .. }
                | MessageSync::UpsertRoomMember { .. }
                | MessageSync::UpsertThreadMember { .. }
                | MessageSync::UpsertRole { .. }
                | MessageSync::UpsertInvite { .. }
                | MessageSync::DeleteMessage { .. }
                | MessageSync::DeleteMessageVersion { .. }
                | MessageSync::DeleteRole { .. }
                | MessageSync::DeleteInvite { .. }
                | MessageSync::ReactionMessagePurge { .. }
                | MessageSync::ReactionThreadPurge { .. }
        )
    }

    // pub fn is_thread_audit_loggable(&self) -> bool {
    //     todo!()
    // }

    /// get id to populate payload_prev
    pub fn get_audit_target_id(&self) -> Option<String> {
        match self {
            MessageSync::UpsertRoom { room } => Some(room.id.to_string()),
            MessageSync::UpsertThread { thread } => Some(thread.id.to_string()),
            MessageSync::UpsertMessage { message } => Some(message.id.to_string()),
            MessageSync::UpsertRoomMember { member } => Some(member.user_id.to_string()),
            MessageSync::UpsertRole { role } => Some(role.id.to_string()),
            MessageSync::UpsertInvite { invite } => Some(invite.invite.code.to_string()),
            MessageSync::DeleteRole { role_id, .. } => Some(role_id.to_string()),
            MessageSync::DeleteInvite { code, .. } => Some(code.to_string()),
            MessageSync::DeleteMessage { message_id, .. } => Some(message_id.to_string()),
            MessageSync::DeleteMessageVersion { message_id, .. } => Some(message_id.to_string()),

            // HACK: prob. should impl thread-specific audit logs?
            MessageSync::UpsertThreadMember { member } => {
                Some(format!("{}-{}", member.user_id, member.thread_id))
            }

            // not loggable
            _ => None,
        }
    }
}
