use async_trait::async_trait;
use common::v1::types::{
    util::Time, InviteCode, InviteWithMetadata, Message, MessageId, MessagePayload, MessageSync,
    MessageVerId, Role, RoleId, Room, RoomId, RoomMember, Session, SessionId, Thread, ThreadId,
    ThreadMember, User, UserId,
};
use std::future::{ready, Future};

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

    fn upsert_thread_member(
        &mut self,
        member: ThreadMember,
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

    fn delete_invite(
        &mut self,
        code: InviteCode,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn typing(
        &mut self,
        thread_id: ThreadId,
        user_id: UserId,
        until: Time,
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
                MessageSync::RoomCreate { room } => self.upsert_room(room).await,
                MessageSync::ThreadCreate { thread } => self.upsert_thread(thread).await,
                MessageSync::MessageCreate { message } => self.upsert_message(message).await,
                MessageSync::UserCreate { user } => self.upsert_user(user).await,
                MessageSync::RoomMemberUpsert { member } => self.upsert_room_member(member).await,
                MessageSync::ThreadMemberUpsert { member } => {
                    self.upsert_thread_member(member).await
                }
                MessageSync::SessionCreate { session } => self.upsert_session(session).await,
                MessageSync::RoleCreate { role } => self.upsert_role(role).await,
                MessageSync::InviteCreate { invite } => self.upsert_invite(invite).await,
                MessageSync::MessageDelete {
                    thread_id,
                    message_id,
                    ..
                } => self.delete_message(thread_id, message_id).await,
                MessageSync::MessageVersionDelete {
                    thread_id,
                    message_id,
                    version_id,
                    ..
                } => {
                    self.delete_message_version(thread_id, message_id, version_id)
                        .await
                }
                MessageSync::UserDelete { id } => self.delete_user(id).await,
                MessageSync::SessionDelete { id, .. } => self.delete_session(id).await,
                MessageSync::RoleDelete { room_id, role_id } => {
                    self.delete_role(room_id, role_id).await
                }
                MessageSync::InviteDelete { code, .. } => self.delete_invite(code).await,
                MessageSync::ThreadTyping {
                    thread_id,
                    user_id,
                    until,
                } => self.typing(thread_id, user_id, until).await,
                _ => todo!(),
            },
            MessagePayload::Error { error } => self.error(error).await,
            MessagePayload::Ready { user, session, .. } => self.ready(user, session).await,
            _ => return,
        };
    }
}
