use serde::{Deserialize, Serialize};

use serde_json::Value;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{
    user_status::StatusPatch, util::Time, InviteTargetId, InviteWithMetadata, Relationship,
    ThreadMember,
};

use super::{
    emoji::EmojiCustom,
    reaction::ReactionKey,
    voice::{SignallingMessage, VoiceState},
    EmojiId, InviteCode, Message, MessageId, MessageVerId, Role, RoleId, Room, RoomId, RoomMember,
    Session, SessionId, SessionToken, Thread, ThreadId, User, UserId,
};

mod sync2;

pub use sync2::{SyncCompression, SyncFormat, SyncParams, SyncVersion};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    #[cfg(feature = "feat_voice")]
    /// send arbitrary data to a voice server
    // TEMP: for prototyping
    VoiceDispatch {
        user_id: UserId,
        // TODO: multiple servers
        // server_id: ServerId,
        payload: Value,
    },
    // #[cfg(feature = "feat_voice")]
    // /// connect or disconnect to a voice channel
    // VoiceState {
    //     thread_id: Option<ThreadId>,
    // },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncResume {
    pub conn: String,
    pub seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageEnvelope {
    #[serde(flatten)]
    pub payload: MessagePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
#[allow(clippy::large_enum_variant)]
pub enum MessageSync {
    RoomCreate {
        room: Room,
    },

    RoomUpdate {
        room: Room,
    },

    ThreadCreate {
        thread: Thread,
    },

    ThreadUpdate {
        thread: Thread,
    },

    ThreadTyping {
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

    MessageCreate {
        message: Message,
    },

    MessageUpdate {
        message: Message,
    },

    MessageDelete {
        /// deprecated = "keyed by thread_id"
        #[cfg_attr(feature = "utoipa", schema(deprecated))]
        room_id: RoomId,
        thread_id: ThreadId,
        message_id: MessageId,
    },

    MessageVersionDelete {
        /// deprecated = "keyed by thread_id"
        #[cfg_attr(feature = "utoipa", schema(deprecated))]
        room_id: RoomId,
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    },

    /// remove multiple messages at once
    MessageDeleteBulk {
        thread_id: ThreadId,
        message_ids: Vec<MessageId>,
    },

    RoomMemberUpsert {
        member: RoomMember,
    },

    ThreadMemberUpsert {
        member: ThreadMember,
    },

    RoleCreate {
        role: Role,
    },

    RoleUpdate {
        role: Role,
    },

    RoleDelete {
        room_id: RoomId,
        role_id: RoleId,
    },

    InviteCreate {
        invite: InviteWithMetadata,
    },

    // InviteUpdate {
    //     invite: InviteWithMetadata,
    // },
    InviteDelete {
        code: InviteCode,
        target: InviteTargetId,
    },

    ReactionCreate {
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    },

    ReactionDelete {
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    },

    /// remove all reactions
    ReactionPurge {
        thread_id: ThreadId,
        message_id: MessageId,
    },

    EmojiCreate {
        emoji: EmojiCustom,
    },

    EmojiDelete {
        emoji_id: EmojiId,
        room_id: RoomId,
    },

    #[cfg(feature = "feat_voice")]
    /// receive arbitrary data from a voice server
    // TEMP: for prototyping
    VoiceDispatch {
        user_id: UserId,
        // TODO: multiple servers
        // server_id: ServerId,
        payload: SignallingMessage,
    },

    VoiceState {
        user_id: UserId,
        state: Option<VoiceState>,
    },

    UserCreate {
        user: User,
    },

    UserUpdate {
        user: User,
    },

    UserDelete {
        id: UserId,
    },

    SessionCreate {
        session: Session,
    },

    SessionUpdate {
        session: Session,
    },

    SessionDelete {
        id: SessionId,
        user_id: Option<UserId>,
    },

    RelationshipUpsert {
        user_id: UserId,
        relationship: Relationship,
    },

    RelationshipDelete {
        user_id: UserId,
    },
    // /// arbitrary user defined event
    // Dispatch {
    //     user_id: UserId,
    //     action: String,
    //     payload: Option<serde_json::Value>,
    // },
}

impl MessageSync {
    pub fn is_room_audit_loggable(&self) -> bool {
        matches!(
            self,
            MessageSync::RoomCreate { .. }
                | MessageSync::RoomUpdate { .. }
                | MessageSync::ThreadCreate { .. }
                | MessageSync::ThreadUpdate { .. }
                | MessageSync::RoomMemberUpsert { .. }
                | MessageSync::ThreadMemberUpsert { .. }
                | MessageSync::RoleCreate { .. }
                | MessageSync::RoleUpdate { .. }
                | MessageSync::RoleDelete { .. }
                | MessageSync::InviteCreate { .. }
                | MessageSync::MessageDelete { .. }
                | MessageSync::MessageVersionDelete { .. }
                | MessageSync::InviteDelete { .. }
                | MessageSync::ReactionPurge { .. }
                | MessageSync::EmojiCreate { .. }
                | MessageSync::EmojiDelete { .. }
        )
    }

    // pub fn is_thread_audit_loggable(&self) -> bool {
    //     todo!()
    // }

    /// get id to populate payload_prev
    pub fn get_audit_target_id(&self) -> Option<String> {
        match self {
            MessageSync::RoomCreate { room } => Some(room.id.to_string()),
            MessageSync::RoomUpdate { room } => Some(room.id.to_string()),
            MessageSync::ThreadCreate { thread } => Some(thread.id.to_string()),
            MessageSync::ThreadUpdate { thread } => Some(thread.id.to_string()),
            MessageSync::MessageCreate { message } => Some(message.id.to_string()),
            MessageSync::MessageUpdate { message } => Some(message.id.to_string()),
            MessageSync::RoomMemberUpsert { member } => Some(member.user_id.to_string()),
            MessageSync::RoleCreate { role } => Some(role.id.to_string()),
            MessageSync::RoleUpdate { role } => Some(role.id.to_string()),
            MessageSync::RoleDelete { role_id, .. } => Some(role_id.to_string()),
            MessageSync::InviteCreate { invite } => Some(invite.invite.code.to_string()),
            MessageSync::InviteDelete { code, .. } => Some(code.to_string()),
            MessageSync::MessageDelete { message_id, .. } => Some(message_id.to_string()),
            MessageSync::MessageVersionDelete { message_id, .. } => Some(message_id.to_string()),
            MessageSync::EmojiCreate { emoji } => Some(emoji.id.to_string()),
            MessageSync::EmojiDelete { emoji_id, .. } => Some(emoji_id.to_string()),

            // HACK: prob. should impl thread-specific audit logs?
            MessageSync::ThreadMemberUpsert { member } => {
                Some(format!("{}-{}", member.user_id, member.thread_id))
            }

            // not loggable
            _ => None,
        }
    }
}
