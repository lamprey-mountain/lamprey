use async_trait::async_trait;
use std::future::{ready, Future};
use types::{
    InviteCode, InviteWithMetadata, Message, MessageId, MessagePayload, MessageSync, MessageVerId,
    Role, RoleId, Room, RoomId, RoomMember, Session, SessionId, Thread, ThreadId, User, UserId,
};
use uuid::Uuid;

#[allow(unused_variables)]
pub trait EventHandler: Send {
    type Error: Send;

    fn ready(
        &mut self,
        user: Option<User>,
        session: Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn error(&mut self, err: String) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn upsert_room(&mut self, room: Room) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn upsert_thread(
        &mut self,
        thread: Thread,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn upsert_message(
        &mut self,
        message: Message,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn upsert_user(&mut self, user: User) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn upsert_room_member(
        &mut self,
        member: RoomMember,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn upsert_session(
        &mut self,
        session: Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn upsert_role(&mut self, role: Role) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn upsert_invite(
        &mut self,
        invite: InviteWithMetadata,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn delete_message(
        &mut self,
        thread_id: ThreadId,
        message_id: MessageId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn delete_message_version(
        &mut self,
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn delete_user(&mut self, id: UserId) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn delete_session(
        &mut self,
        id: SessionId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn delete_role(
        &mut self,
        room_id: RoomId,
        role_id: RoleId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn delete_room_member(
        &mut self,
        room_id: RoomId,
        user_id: UserId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn delete_invite(
        &mut self,
        code: InviteCode,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn webhook(
        &mut self,
        hook_id: Uuid,
        data: serde_json::Value,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }
}

pub struct EmptyHandler;

impl EventHandler for EmptyHandler {
    type Error = ();
}

#[async_trait]
pub trait ErasedHandler: Send {
    async fn handle(&mut self, payload: MessagePayload);
}

#[async_trait]
impl<T, E> ErasedHandler for T
where
    T: EventHandler<Error = E>,
{
    async fn handle(&mut self, payload: MessagePayload) {
        let _ = match payload {
            MessagePayload::Sync { data, .. } => match data {
                MessageSync::UpsertRoom { room } => self.upsert_room(room).await,
                MessageSync::UpsertThread { thread } => self.upsert_thread(thread).await,
                MessageSync::UpsertMessage { message } => self.upsert_message(message).await,
                MessageSync::UpsertUser { user } => self.upsert_user(user).await,
                MessageSync::UpsertRoomMember { member } => self.upsert_room_member(member).await,
                MessageSync::UpsertSession { session } => self.upsert_session(session).await,
                MessageSync::UpsertRole { role } => self.upsert_role(role).await,
                MessageSync::UpsertInvite { invite } => self.upsert_invite(invite).await,
                MessageSync::DeleteMessage {
                    thread_id,
                    message_id,
                } => self.delete_message(thread_id, message_id).await,
                MessageSync::DeleteMessageVersion {
                    thread_id,
                    message_id,
                    version_id,
                } => {
                    self.delete_message_version(thread_id, message_id, version_id)
                        .await
                }
                MessageSync::DeleteUser { id } => self.delete_user(id).await,
                MessageSync::DeleteSession { id, .. } => self.delete_session(id).await,
                MessageSync::DeleteRole { room_id, role_id } => {
                    self.delete_role(room_id, role_id).await
                }
                MessageSync::DeleteRoomMember { room_id, user_id } => {
                    self.delete_room_member(room_id, user_id).await
                }
                MessageSync::DeleteInvite { code, .. } => self.delete_invite(code).await,
                MessageSync::Webhook { hook_id, data } => self.webhook(hook_id, data).await,
            },
            MessagePayload::Error { error } => self.error(error).await,
            MessagePayload::Ready { user, session, .. } => self.ready(user, session).await,
            _ => return,
        };
    }
}
