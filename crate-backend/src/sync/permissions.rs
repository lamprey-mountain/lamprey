// TODO: refactor out permission calculator

use std::sync::Arc;

use common::v1::types::{ChannelId, MessageSync, Permission, RoomId, UserId};

use crate::{sync::ConnectionState, Result, ServerState};

#[derive(Debug)]
pub enum AuthCheck {
    Custom(bool),
    Room(RoomId),
    RoomPerm(RoomId, Permission),
    RoomOrUser(RoomId, UserId),
    ChannelOrUser(ChannelId, UserId),
    User(UserId),
    UserMutual(UserId),
    Channel(ChannelId),
    ChannelPerm(ChannelId, Permission),
    EitherChannel(ChannelId, ChannelId),
}
