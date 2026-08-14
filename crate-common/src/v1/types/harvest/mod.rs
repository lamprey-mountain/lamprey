use lamprey_macros::record;

use crate::{
    v1::types::{HarvestId, RoomId, UserId, misc::Time},
    v2::types::media::Media,
};

pub mod inside;

/// how to create a harvest
///
/// including extra data will make the export slower
#[record]
pub struct HarvestCreateUser {
    /// include all messages you have sent
    pub include_messages: bool,

    /// include all reactions you have sent
    pub include_reactions: bool,
}

/// how to create a harvest
///
/// including extra data will make the export slower
#[record]
pub struct HarvestCreateRoom {
    /// include all messages in this room
    pub include_messages: bool,

    /// include all individual reactions
    ///
    /// otherwise only include counts
    pub include_reactions: bool,

    /// include all members in this room
    pub include_members: bool,
}

#[record]
pub struct Harvest {
    pub id: HarvestId,

    /// user who requested this harvest to be generated
    pub requester_id: UserId,

    /// when this archive was created
    pub queued_at: Time,

    #[serde(flatten)]
    pub status: HarvestStatus,

    #[serde(flatten)]
    pub ty: HarvestType,
}

#[record]
#[derive(PartialEq, Eq)]
#[serde(tag = "status")]
pub enum HarvestStatus {
    /// this is queued and not running yet
    Queued,

    /// archiving in progress
    Archiving {
        /// when archiving started
        started_at: Time,

        /// estimated time this archive is done
        eta_at: Option<Time>,
    },

    /// the export failed, contact support for help
    Failed {
        failed_at: Time,
        code: HarvestFailedCode,
        message: String,
    },

    /// the export completed successfully
    Completed {
        started_at: Time,
        completed_at: Time,
        expires_at: Time,
        media: Media,
    },

    /// the export was cancelled. try again, contact support if this keeps happening?
    Cancelled {
        cancelled_at: Time,
        message: String,
        reason: HarvestCancelReason,
    },
}

/// the reason why harvest generation failed
#[record]
#[derive(PartialEq, Eq)]
pub enum HarvestFailedCode {
    // TODO: remove
    Other,
}

/// the reason why harvest generation was cancelled
#[record]
#[derive(PartialEq, Eq)]
pub enum HarvestCancelReason {
    CancelledByUser,
    CancelledByAdmin,

    // TODO: remove
    Other,
}

/// what this harvest was generated for
#[record]
#[serde(tag = "type")]
pub enum HarvestType {
    User {
        target_user_id: UserId,

        /// the create options
        create: HarvestCreateUser,
    },

    Room {
        target_room_id: RoomId,

        /// the create options
        create: HarvestCreateRoom,
    },
}
