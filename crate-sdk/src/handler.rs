use std::future::{ready, Future};
use types::{
    Invite, InviteCode, Message, MessageId, MessageVerId, Role, RoleId, Room, RoomId, RoomMember,
    Session, SessionId, Thread, ThreadId, User, UserId,
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

    fn upsert_member(
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
        invite: Invite,
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

    fn delete_member(
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
