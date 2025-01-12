use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid7::Uuid;

use super::{
    Invite, InviteCode, Member, Message, MessageId, MessageVersionId, Role, RoleId, Room, RoomId,
    Session, SessionId, Thread, User, UserId,
};

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageClient {
    Hello {
        token: String,

        // TODO: resutming connections,
        last_id: Option<String>,
    },
    Pong,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
#[serde(tag = "type")]
enum MessageServer {
    #[serde(rename = "ping")]
    Ping {},
    #[serde(rename = "ready")]
    Ready { user: User },
    #[serde(rename = "error")]
    Error { error: String },
    #[serde(rename = "upsert.room")]
    UpsertRoom { room: Room },
    #[serde(rename = "upsert.thread")]
    UpsertThread { thread: Thread },
    #[serde(rename = "upsert.message")]
    UpsertMessage { message: Message },
    #[serde(rename = "upsert.user")]
    UpsertUser { user: User },
    #[serde(rename = "upsert.member")]
    UpsertMember { member: Member },
    #[serde(rename = "upsert.session")]
    UpsertSession { session: Session },
    #[serde(rename = "upsert.role")]
    UpsertRole { role: Role },
    #[serde(rename = "upsert.invite")]
    UpsertInvite { invite: Invite },
    #[serde(rename = "delete.message")]
    DeleteMessage { id: MessageId },
    #[serde(rename = "delete.message_version")]
    DeleteMessageVersion { id: MessageVersionId },
    #[serde(rename = "delete.user")]
    DeleteUser { id: UserId },
    #[serde(rename = "delete.session")]
    DeleteSession { id: SessionId },
    #[serde(rename = "delete.role")]
    DeleteRole { id: RoleId },
    #[serde(rename = "delete.member")]
    DeleteMember { room_id: RoomId, user_id: UserId },
    #[serde(rename = "delete.invite")]
    DeleteInvite { code: InviteCode },
    #[serde(rename = "webhook")]
    Webhook {
        hook_id: Uuid,
        data: serde_json::Value,
    },
}
