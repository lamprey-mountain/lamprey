//! Lamprey actor message types and responses

use std::sync::Arc;

use common::v1::types::pagination::{PaginationQuery, PaginationResponse};
use common::v1::types::util::Time;
use common::v1::types::{
    self, presence, Channel, ChannelId, ChannelType, MessageCreate, MessageId, RoomId, User, UserId,
};
use common::v2::types::media::Media;
use common::v2::types::message::Message;

/// Lamprey actor messages - request/response pattern
#[derive(Debug)]
pub enum LampreyMessage {
    MediaUpload {
        filename: String,
        bytes: Vec<u8>,
        user_id: UserId,
    },
    MessageGet {
        thread_id: ChannelId,
        message_id: MessageId,
    },
    MessageList {
        thread_id: ChannelId,
        query: Arc<PaginationQuery<MessageId>>,
    },
    MessageCreate {
        thread_id: ChannelId,
        user_id: UserId,
        req: MessageCreate,
    },
    MessageCreateWithTimestamp {
        thread_id: ChannelId,
        user_id: UserId,
        req: MessageCreate,
        timestamp: Time,
    },
    MessageUpdate {
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        req: types::MessagePatch,
    },
    MessageDelete {
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
    },
    MessageReact {
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        reaction: String,
    },
    MessageUnreact {
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        reaction: String,
    },
    TypingStart {
        thread_id: ChannelId,
        user_id: UserId,
    },
    PuppetEnsure {
        name: String,
        key: String,
        room_id: RoomId,
        bot: bool,
    },
    UserFetch {
        user_id: UserId,
    },
    UserUpdate {
        user_id: UserId,
        patch: types::UserPatch,
    },
    UserSetPresence {
        user_id: UserId,
        patch: presence::Presence,
    },
    RoomMemberPatch {
        room_id: RoomId,
        user_id: UserId,
        patch: types::RoomMemberPatch,
    },
    RoomThreads {
        room_id: RoomId,
    },
    CreateThread {
        room_id: RoomId,
        name: String,
        topic: Option<String>,
        ty: ChannelType,
        parent_id: Option<ChannelId>,
    },
}

/// Response types for LampreyMessage requests
#[derive(Debug)]
pub enum LampreyResponse {
    Media(Media),
    Message(Message),
    MessageList(PaginationResponse<Message>),
    User(User),
    RoomMember(types::RoomMember),
    RoomThreads(Vec<Channel>),
    Channel(Channel),
    Empty,
}
