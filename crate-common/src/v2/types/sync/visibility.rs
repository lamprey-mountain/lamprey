use crate::{
    v1::types::{ChannelId, ConnectionId, Permission, RoomId, SessionId, UserId},
    v2::types::sync::Dispatch,
};

/// the visibility of this dispatch
#[derive(Debug)]
pub enum DispatchVisibility {
    /// must be able to view this room
    ///
    /// allows lurkers
    Room(RoomId),

    /// must have this permission in this room
    ///
    /// allows lurkers
    RoomPerm(RoomId, Permission),

    /// must be able to view this channel
    ///
    /// allows lurkers
    Channel(ChannelId),

    /// must have this permission in this channel
    ///
    /// allows lurkers
    ChannelPerm(ChannelId, Permission),

    /// must be this user
    User(UserId),

    /// must be able to see this user
    ///
    /// - friends
    /// - mutual rooms
    /// - mutual gdms
    UserVisible(UserId),

    /// must be this session
    Session(SessionId),

    /// must be this connection
    Connection(ConnectionId),

    /// any of these checks must pass
    AnyOf(Vec<DispatchVisibility>),
}

impl DispatchVisibility {
    /// return an auth check for "either in this room or is this user"
    fn room_or_user(room_id: RoomId, user_id: UserId) -> Self {
        DispatchVisibility::AnyOf(vec![
            DispatchVisibility::Room(room_id),
            DispatchVisibility::User(user_id),
        ])
    }
}

impl Dispatch {
    /// get the visibility check for this dispatch
    pub fn visibility(&self) -> DispatchVisibility {
        // TODO: copy logic from crate-backend/src/sync/permissions.rs
        match self {
            Dispatch::Ready { connection_id, .. } => DispatchVisibility::Connection(*connection_id),
            Dispatch::Ambient { connection_id, .. } => {
                DispatchVisibility::Connection(*connection_id)
            }
            Dispatch::VoiceDispatch { user_id, .. } => DispatchVisibility::User(*user_id),
            Dispatch::VoiceState { .. } => todo!(),
            Dispatch::DocumentEdit { .. } => todo!(),
            Dispatch::DocumentPresence { .. } => todo!(),
            Dispatch::MediaProcessed { session_id, .. } => DispatchVisibility::Session(*session_id),
            Dispatch::MediaUpdate { .. } => todo!(),
            Dispatch::Room(d) => match &d.inner {
                // DispatchRoomInner::AuditLogEntryCreate { .. } => todo!(),
                _ => DispatchVisibility::Room(d.room_id),
            },
            Dispatch::Channel(d) => match &d.inner {
                // DispatchChannelInner::
                _ => todo!(),
            },
            Dispatch::User(_d) => todo!(),
            Dispatch::Subscriptions(_d) => todo!(),
            Dispatch::Invite(d) => match &d.target {
                _ => todo!(),
            },
            Dispatch::Webhook(_d) => todo!(),
        }
    }
}
