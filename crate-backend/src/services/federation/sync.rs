use std::collections::HashMap;
use std::sync::Arc;

use crate::error::Result;
use crate::services::federation::ServiceFederation;
use crate::services::rooms::RoomData;
use crate::sync::permissions::AuthCheck;
use common::v1::types::federation::{Hostname, ServerSyncRequest};
use common::v1::types::{ChannelId, MessageSync, RoomId, UserId};

impl ServiceFederation {
    /// handle a sync from a remote server
    pub async fn handle_sync(&self, req: ServerSyncRequest) -> Result<()> {
        // TODO: update database
        // TODO: broadcast sync event
        Ok(())
    }

    async fn update_database(&self, msg: MessageSync) -> Result<()> {
        todo!()
    }

    /// push a sync event to any known remote servers
    pub async fn push_sync(&self, msg: MessageSync) -> Result<()> {
        // PERF: aggregate n sync events and send as a batch instead of sending one at a time
        // TODO: filter visibility with ServerPerspective
        // TODO: subscribe to sushi belt and send it to push_sync
        // TODO: store a list of connected servers in ServiceFederation
        // TODO: persist the list of connected servers and load it on startup
        todo!()
    }

    /// get a `ServerPerspective` for the host
    async fn load_perspective(&self, hostname: Hostname) -> Result<ServerPerspective> {
        // PERF: how much can i cache perspectives?
        todo!()
    }
}

/// what a server is able to see
// NOTE: load all relevant data into one big struct for now
pub struct ServerPerspective {
    /// room data for all the rooms the server is participating in (including members)
    pub room_data: HashMap<RoomId, Arc<RoomData>>,
    // room members from the server
    // how do i handle dms/gdms? include every non-room channel the server is participating in
    // send user/presence updates for friend requests? what about non-friend relationships, like blocked users?
}

impl ServerPerspective {
    /// whether this server can view this sync event
    // NOTE: maybe use passes_auth_check instead of reimplementing AuthCheck::for_message
    pub fn can_view_sync(&self, msg: &MessageSync) -> bool {
        match msg {
            MessageSync::UserCreate { user } => self.can_view_user(user.id),
            MessageSync::UserUpdate { user } => self.can_view_user(user.id),
            MessageSync::UserDelete { id } => self.can_view_user(*id),

            MessageSync::PresenceUpdate { user_id, .. } => self.can_view_user(*user_id),

            // TODO: handle more events
            _ => false,
        }
    }

    /// whether this server can view this user
    pub fn can_view_user(&self, user_id: UserId) -> bool {
        // true iff any user from this server (user.remote)
        // - shares a mutual room with the target user
        // - is friends the target user
        todo!()
    }

    /// whether this server can view this room
    pub fn can_view_room(&self, room_id: RoomId) -> bool {
        // true iff the server has a room member in that room (check user.remote)
        todo!()
    }

    /// whether this server can view this channel
    pub fn can_view_channel(&self, channel_id: ChannelId) -> bool {
        // true iff the server has any user with viewchannel perms for that channel
        todo!()
    }

    pub fn passes_auth_check(&self, check: AuthCheck) -> bool {
        match check {
            AuthCheck::Room(id) => self.can_view_room(id),
            AuthCheck::RoomPerm(id, permission) => todo!("how does this work?"),
            AuthCheck::Channel(id) => self.can_view_channel(id),
            AuthCheck::ChannelPerm(id, permission) => todo!("how does this work?"),
            AuthCheck::User(id) => todo!("true if this user exists on the remote server"),
            AuthCheck::UserVisible(id) => self.can_view_user(id),
            AuthCheck::Session(_) | AuthCheck::Connection(_) => false,
            AuthCheck::Any(auth_checks) => todo!(),
        }
    }
}
