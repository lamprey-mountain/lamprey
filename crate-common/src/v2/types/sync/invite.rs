use lamprey_macros::record;

use crate::v1::types::{Invite, InviteCode, InviteTargetId, RoomId};

/// something happened with an invite
#[record]
pub struct DispatchInvite {
    pub invite_code: InviteCode,

    /// the room this invite belongs to, if any
    pub room_id: Option<RoomId>,

    /// the target of this invite
    ///
    /// used to determine which scope (room/channel) this dispatch belongs to
    pub target: InviteTargetId,

    #[serde(flatten)]
    pub inner: DispatchInviteInner,
}

#[record]
#[serde(tag = "type")]
pub enum DispatchInviteInner {
    /// an invite was created
    InviteCreate { invite: Box<Invite> },

    /// an invite was updated
    InviteUpdate { invite: Box<Invite> },

    /// an invite was deleted
    InviteDelete {
        invite_code: InviteCode,
        target: InviteTargetId,
    },
}
