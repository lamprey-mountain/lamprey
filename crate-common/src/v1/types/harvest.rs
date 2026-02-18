#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{misc::Time, HarvestId, UserId};

/// how to create a harvest
///
/// including extra data will make the export slower
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct HarvestCreate {
    /// include all messages you have sent
    pub include_messages: bool,

    /// include all reactions you have sent
    pub include_reactions: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Harvest {
    pub id: HarvestId,
    pub user_id: UserId,
    pub created_at: Time,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub status: HarvestStatus,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "status"))]
pub enum HarvestStatus {
    /// this is in progress or is running
    Queued,

    /// the export failed, contact support for help
    Failed { failed_at: Time, message: String },

    /// the export completed successfully
    Completed {
        completed_at: Time,
        url: Url,
        expires_at: Time,
    },

    /// the export was cancelled. try again, contact support if this keeps happening?
    // reason: cancelled by user | cancelled by staff
    Cancelled { cancelled_at: Time, message: String },
}
