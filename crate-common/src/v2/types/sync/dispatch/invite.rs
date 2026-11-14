use lamprey_macros::record;

use crate::v1::types::{InviteCode, InviteTargetId, InviteWithMetadata};

/// an invite was created
#[record]
pub struct InviteCreate {
    pub invite: Box<InviteWithMetadata>,
    pub target: InviteTargetId,
}

/// an invite was updated
#[record]
pub struct InviteUpdate {
    pub invite: Box<InviteWithMetadata>,
    pub target: InviteTargetId,
}

/// an invite was deleted
#[record]
pub struct InviteDelete {
    pub invite_code: InviteCode,
    pub target: InviteTargetId,
}

/// all invites for a target were deleted
#[record]
pub struct InviteDeleteBulk {
    pub target: InviteTargetId,
    pub codes: Vec<InviteCode>,
}
