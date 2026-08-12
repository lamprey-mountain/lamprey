// TODO: clean up/remove some of these structs

use lamprey_macros::record;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{ChannelId, SessionId, UserId};

/// a mls epoch number, incremented each time the group membership changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MlsEpoch(pub u64);

// NOTE: im not sure how necessary having the below structs are?

/// a mls key package, uploaded by sessions for use in welcomes
#[record]
pub struct MlsKeyPackage {
    pub user_id: UserId,
    pub session_id: SessionId,

    /// opaque mls key package data
    #[validate(length(min = 1, max = 65535))]
    pub data: Vec<u8>,
    // pub data: Binary<65535>,
}

#[record]
pub struct MlsKeyPackageUpload {
    pub packages: Vec<MlsKeyPackage>,
}

#[record]
pub struct MlsKeyPackageClaim {
    pub users: Vec<UserId>,
}

/// a welcome message used to add a new member to an mls group
#[record]
pub struct MlsWelcome {
    /// the opaque welcome message data (MLS Welcome message).
    #[validate(length(min = 1, max = 4194304))]
    pub data: Vec<u8>,

    /// the session that sent this welcome
    pub sender_id: SessionId,

    /// the channel (mls group) to join
    pub channel_id: ChannelId,
}

#[record]
pub struct MlsWelcomeCreate {
    #[validate(length(min = 1, max = 4194304))]
    pub data: Vec<u8>,
}

/// an mls commit message, representing group state changes.
#[record]
pub struct MlsCommit {
    /// opaque data
    ///
    /// is a commit or proposal for member add, remove, update
    #[validate(length(min = 1, max = 4194304))]
    pub data: Vec<u8>,

    /// the session that authored this message
    pub sender_id: SessionId,

    /// the channel (mls group) this takes place in
    pub channel_id: ChannelId,
}

#[record]
pub struct MlsCommitCreate {
    #[validate(length(min = 1, max = 4194304))]
    pub data: Vec<u8>,
}

#[record]
pub struct MlsMessageCreate {
    #[validate(length(min = 1, max = 4194304))]
    pub data: Vec<u8>,
}

#[record]
pub struct MlsEpochCreate {
    #[validate(length(min = 1, max = 4194304))]
    pub data: Vec<u8>,
    pub epoch: MlsEpoch,
}
