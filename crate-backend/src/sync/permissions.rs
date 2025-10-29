// Permission calculator and authorization checks

use std::sync::Arc;

use common::v1::types::{ChannelId, Permission, RoomId, Session, UserId};

use crate::{Result, ServerState};

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

impl AuthCheck {
    pub async fn should_send(
        &self,
        session: &Session,
        server_state: &Arc<ServerState>,
    ) -> Result<bool> {
        let should_send = match (session.user_id(), self) {
            (Some(user_id), AuthCheck::Room(room_id)) => {
                let _perms = server_state
                    .services()
                    .perms
                    .for_room(user_id, *room_id)
                    .await?;
                true
            }
            (Some(user_id), AuthCheck::RoomPerm(room_id, perm)) => {
                let perms = server_state
                    .services()
                    .perms
                    .for_room(user_id, *room_id)
                    .await?;
                perms.has(*perm)
            }
            (Some(auth_user_id), AuthCheck::RoomOrUser(room_id, target_user_id)) => {
                if auth_user_id == *target_user_id {
                    true
                } else {
                    let _perms = server_state
                        .services()
                        .perms
                        .for_room(auth_user_id, *room_id)
                        .await?;
                    true
                }
            }
            (Some(user_id), AuthCheck::Channel(thread_id)) => {
                let perms = server_state
                    .services()
                    .perms
                    .for_channel(user_id, *thread_id)
                    .await?;
                perms.has(Permission::ViewChannel)
            }
            (Some(user_id), AuthCheck::ChannelPerm(thread_id, perm)) => {
                let perms = server_state
                    .services()
                    .perms
                    .for_channel(user_id, *thread_id)
                    .await?;
                perms.has(Permission::ViewChannel) && perms.has(*perm)
            }
            (Some(user_id), AuthCheck::EitherChannel(thread_id_0, thread_id_1)) => {
                let perms0 = server_state
                    .services()
                    .perms
                    .for_channel(user_id, *thread_id_0)
                    .await?;
                let perms1 = server_state
                    .services()
                    .perms
                    .for_channel(user_id, *thread_id_1)
                    .await?;
                perms0.has(Permission::ViewChannel) || perms1.has(Permission::ViewChannel)
            }
            (Some(auth_user_id), AuthCheck::ChannelOrUser(thread_id, target_user_id)) => {
                if auth_user_id == *target_user_id {
                    true
                } else {
                    let perms = server_state
                        .services()
                        .perms
                        .for_channel(auth_user_id, *thread_id)
                        .await?;
                    perms.has(Permission::ViewChannel)
                }
            }
            (Some(auth_user_id), AuthCheck::User(target_user_id)) => {
                auth_user_id == *target_user_id
            }
            (Some(auth_user_id), AuthCheck::UserMutual(target_user_id)) => {
                if auth_user_id == *target_user_id {
                    true
                } else {
                    server_state
                        .services()
                        .perms
                        .is_mutual(auth_user_id, *target_user_id)
                        .await?
                }
            }
            (_, AuthCheck::Custom(b)) => *b,
            (None, _) => false,
        };
        Ok(should_send)
    }
}
