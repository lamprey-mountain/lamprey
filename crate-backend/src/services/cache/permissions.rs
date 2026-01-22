//! permission calculations for cached rooms

use common::v1::types::{ChannelId, UserId};

use crate::{services::cache::CachedRoom, types::Permissions};

impl CachedRoom {
    /// query permissions for a room member
    pub fn query_permissions(&self, user_id: UserId, channel_id: Option<ChannelId>) -> Permissions {
        let Some(member) = self.members.get(&user_id) else {
            if self.room.public {
                // use public/default perms
                todo!()
            } else {
                // no perms
                todo!()
            }
        };

        // calculate base perms
        // let perms = ...

        if let Some(channel_id) = channel_id {
            // TODO: calculate permission overwrites
            // TODO: handle threads (only one layer of nesting, so no need to recursively lookup)
            todo!()
        } else {
            // return room permissions
            todo!()
        }
    }
}
