use async_trait::async_trait;
use common::v1::types::{
    user_config::UserConfig, util::Time, voice::SignallingMessage, InviteCode, InviteWithMetadata,
    Message, MessageId, MessagePayload, MessageSync, MessageVerId, Role, RoleId, Room, RoomId,
    RoomMember, Session, SessionId, Thread, ThreadId, ThreadMember, User, UserId,
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

    fn room_create(&mut self, room: Room) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn room_update(&mut self, room: Room) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn thread_create(
        &mut self,
        thread: Thread,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn thread_update(
        &mut self,
        thread: Thread,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn message_create(
        &mut self,
        message: Message,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn message_update(
        &mut self,
        message: Message,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn user_create(&mut self, user: User) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn user_update(&mut self, user: User) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn room_member_upsert(
        &mut self,
        member: RoomMember,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn thread_member_upsert(
        &mut self,
        member: ThreadMember,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn session_create(
        &mut self,
        session: Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn session_update(
        &mut self,
        session: Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn role_create(&mut self, role: Role) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn role_update(&mut self, role: Role) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn invite_create(
        &mut self,
        invite: InviteWithMetadata,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn invite_update(
        &mut self,
        invite: InviteWithMetadata,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn message_delete(
        &mut self,
        thread_id: ThreadId,
        message_id: MessageId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn message_version_delete(
        &mut self,
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn user_delete(&mut self, id: UserId) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn user_config(
        &mut self,
        config: UserConfig,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn session_delete(
        &mut self,
        id: SessionId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn role_delete(
        &mut self,
        room_id: RoomId,
        role_id: RoleId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn invite_delete(
        &mut self,
        code: InviteCode,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn thread_typing(
        &mut self,
        thread_id: ThreadId,
        user_id: UserId,
        until: Time,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn thread_ack(
        &mut self,
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn message_delete_bulk(
        &mut self,
        thread_id: ThreadId,
        message_ids: Vec<MessageId>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn reaction_create(
        &mut self,
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        key: common::v1::types::reaction::ReactionKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn reaction_delete(
        &mut self,
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        key: common::v1::types::reaction::ReactionKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn reaction_purge(
        &mut self,
        thread_id: ThreadId,
        message_id: MessageId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn emoji_create(
        &mut self,
        emoji: common::v1::types::emoji::EmojiCustom,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn emoji_delete(
        &mut self,
        emoji_id: common::v1::types::EmojiId,
        room_id: RoomId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn voice_dispatch(
        &mut self,
        user_id: UserId,
        payload: common::v1::types::voice::SignallingMessage,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn voice_state(
        &mut self,
        user_id: UserId,
        state: Option<common::v1::types::voice::VoiceState>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn relationship_upsert(
        &mut self,
        user_id: UserId,
        relationship: common::v1::types::Relationship,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn relationship_delete(
        &mut self,
        user_id: UserId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }
}

pub struct EmptyHandler;

impl EventHandler for EmptyHandler {
    type Error = ();

    fn thread_ack(
        &mut self,
        _thread_id: ThreadId,
        _message_id: MessageId,
        _version_id: MessageVerId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn message_delete_bulk(
        &mut self,
        _thread_id: ThreadId,
        _message_ids: Vec<MessageId>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn reaction_create(
        &mut self,
        _user_id: UserId,
        _thread_id: ThreadId,
        _message_id: MessageId,
        _key: common::v1::types::reaction::ReactionKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn reaction_delete(
        &mut self,
        _user_id: UserId,
        _thread_id: ThreadId,
        _message_id: MessageId,
        _key: common::v1::types::reaction::ReactionKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn reaction_purge(
        &mut self,
        _thread_id: ThreadId,
        _message_id: MessageId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn emoji_create(
        &mut self,
        _emoji: common::v1::types::emoji::EmojiCustom,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn emoji_delete(
        &mut self,
        _emoji_id: common::v1::types::EmojiId,
        _room_id: RoomId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn voice_dispatch(
        &mut self,
        _user_id: UserId,
        _payload: SignallingMessage,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn voice_state(
        &mut self,
        _user_id: UserId,
        _state: Option<common::v1::types::voice::VoiceState>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn relationship_upsert(
        &mut self,
        _user_id: UserId,
        _relationship: common::v1::types::Relationship,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn relationship_delete(
        &mut self,
        _user_id: UserId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }
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
                MessageSync::RoomCreate { room } => self.room_create(room).await,
                MessageSync::RoomUpdate { room } => self.room_update(room).await,
                MessageSync::ThreadCreate { thread } => self.thread_create(thread).await,
                MessageSync::ThreadUpdate { thread } => self.thread_update(thread).await,
                MessageSync::MessageCreate { message } => self.message_create(message).await,
                MessageSync::MessageUpdate { message } => self.message_update(message).await,
                MessageSync::UserCreate { user } => self.user_create(user).await,
                MessageSync::UserUpdate { user } => self.user_update(user).await,
                MessageSync::RoomMemberUpsert { member } => self.room_member_upsert(member).await,
                MessageSync::ThreadMemberUpsert { member } => {
                    self.thread_member_upsert(member).await
                }
                MessageSync::SessionCreate { session } => self.session_create(session).await,
                MessageSync::SessionUpdate { session } => self.session_update(session).await,
                MessageSync::RoleCreate { role } => self.role_create(role).await,
                MessageSync::RoleUpdate { role } => self.role_update(role).await,
                MessageSync::InviteCreate { invite } => self.invite_create(invite).await,
                MessageSync::InviteUpdate { invite } => self.invite_update(invite).await,
                MessageSync::MessageDelete {
                    thread_id,
                    message_id,
                    ..
                } => self.message_delete(thread_id, message_id).await,
                MessageSync::MessageVersionDelete {
                    thread_id,
                    message_id,
                    version_id,
                    ..
                } => {
                    self.message_version_delete(thread_id, message_id, version_id)
                        .await
                }
                MessageSync::UserDelete { id } => self.user_delete(id).await,
                MessageSync::UserConfig { user_id: _, config } => self.user_config(config).await,
                MessageSync::SessionDelete { id, .. } => self.session_delete(id).await,
                MessageSync::RoleDelete { room_id, role_id } => {
                    self.role_delete(room_id, role_id).await
                }
                MessageSync::InviteDelete { code, .. } => self.invite_delete(code).await,
                MessageSync::ThreadTyping {
                    thread_id,
                    user_id,
                    until,
                } => self.thread_typing(thread_id, user_id, until).await,
                MessageSync::ThreadAck {
                    thread_id,
                    message_id,
                    version_id,
                } => self.thread_ack(thread_id, message_id, version_id).await,
                MessageSync::MessageDeleteBulk {
                    thread_id,
                    message_ids,
                } => self.message_delete_bulk(thread_id, message_ids).await,
                MessageSync::ReactionCreate {
                    user_id,
                    thread_id,
                    message_id,
                    key,
                } => {
                    self.reaction_create(user_id, thread_id, message_id, key)
                        .await
                }
                MessageSync::ReactionDelete {
                    user_id,
                    thread_id,
                    message_id,
                    key,
                } => {
                    self.reaction_delete(user_id, thread_id, message_id, key)
                        .await
                }
                MessageSync::ReactionPurge {
                    thread_id,
                    message_id,
                } => self.reaction_purge(thread_id, message_id).await,
                MessageSync::EmojiCreate { emoji } => self.emoji_create(emoji).await,
                MessageSync::EmojiDelete { emoji_id, room_id } => {
                    self.emoji_delete(emoji_id, room_id).await
                }
                MessageSync::VoiceDispatch { user_id, payload } => {
                    self.voice_dispatch(user_id, payload).await
                }
                MessageSync::VoiceState { user_id, state, .. } => {
                    self.voice_state(user_id, state).await
                }
                MessageSync::RelationshipUpsert {
                    user_id,
                    relationship,
                } => self.relationship_upsert(user_id, relationship).await,
                MessageSync::RelationshipDelete { user_id } => {
                    self.relationship_delete(user_id).await
                }
            },
            MessagePayload::Error { error } => self.error(error).await,
            MessagePayload::Ready { user, session, .. } => self.ready(user, session).await,
            _ => return,
        };
    }
}
