// TODO: refactor out member list calculator
// should this be moved to services?

use std::sync::Arc;

use common::v1::types::{ChannelId, MessageSync, Role, RoomId, RoomMember};
use dashmap::DashMap;

use crate::{Result, ServerState};

pub struct Lists {
    s: Arc<ServerState>,
    lists: DashMap<RoomId, RoomMembers>,
}

// there should be only one RoomMemers list per room, and the resulting list should NOT be cloned/duplicated excessively
// definitely don't make every connection have a clone of the cached list for diffing
struct RoomMembers {
    roles: Vec<Role>,
    members: Vec<RoomMember>,
}

#[derive(Debug, Clone)]
pub struct MemberListSub {
    pub target: MemberListTarget,
    pub ranges: Vec<(u64, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberListTarget {
    Room(RoomId),
    Channel(ChannelId),
}

impl Lists {
    /// handle a new sync message for a member list
    pub async fn handle(&mut self, message: MessageSync) -> Result<()> {
        todo!()
    }

    /// resync member list from scratch
    pub async fn resync(&mut self, channel_id: ChannelId) -> Result<MessageSync> {
        let srv = self.s.services();
        let chan = srv.channels.get(channel_id, None).await?;

        // copy existing logic here...

        todo!()
    }
}

// lists needs to notify syncers that they need to update the list
// it then needs to calculate a diff for each channel
