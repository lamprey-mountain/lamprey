#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{Invite, InviteCode, InviteTargetId, RoomId};

/// something happened with an invite
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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
