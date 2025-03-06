use serde::{Deserialize, Serialize};

use crate::util::Time;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// #[cfg(feature = "validator")]
// use validator::Validate;

/// the current status of the user
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Status {
    #[serde(flatten)]
    pub status: StatusType,
    #[serde(flatten)]
    pub status_text: Option<StatusText>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct StatusText {
    pub text: String,
    pub clear_at: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum StatusType {
    /// offline or explicitly invisible
    Offline,

    /// connected to the service, no special status
    Online {
        // /// how long this user has been online for
        // online_for: Time,
    },

    /// connected but not currently active (ie away from their computer)
    Away {
        // /// how long this user has been idle for
        // away_for: Time,
    },

    /// currently unavailable to chat
    Busy {
        // /// how long this user will be busy for
        // until: Time,
        /// busy might be set automatically when they look busy
        /// but it might not be that important
        /// this explicitly says "do not disturb"
        dnd: bool,
    },

    /// currently available to chat
    Available {
        // /// how long this user will be available for
        // until: Time,
    },
}

/// an update to a user's status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct StatusPatch {
    #[serde(flatten)]
    pub status: Option<StatusTypePatch>,
    #[serde(flatten)]
    pub status_text: Option<Option<StatusText>>,
}

/// data user sends to update StatusType
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum StatusTypePatch {
    /// offline or explicitly invisible
    Offline,

    /// connected to the service, no special status
    Online,

    /// connected but not currently active (ie away from their computer)
    Away {
        // /// how long this user has been idle for
        // away_for: Time,
    },

    /// currently unavailable to chat
    Busy {
        // /// how long this user will be busy for
        // until: Time,
        /// busy might be set automatically when they look busy
        /// but it might not be that important
        /// this explicitly says "do not disturb"
        dnd: bool,
    },

    /// currently available to chat
    Available {
        // /// how long this user will be available for
        // until: Time,
    },
}

impl Status {
    /// construct a default online status
    pub fn online() -> Status {
        Status {
            status: StatusType::Online {},
            status_text: None,
        }
    }

    /// construct a default offline status
    pub fn offline() -> Status {
        Status {
            status: StatusType::Offline,
            status_text: None,
        }
    }
}

impl StatusPatch {
    // should keep times? idk
    pub fn apply(self, to: Status) -> Status {
        Status {
            status: match self.status {
                Some(StatusTypePatch::Offline) => StatusType::Offline,
                Some(StatusTypePatch::Online) => StatusType::Online {},
                Some(StatusTypePatch::Away {}) => StatusType::Away {},
                Some(StatusTypePatch::Busy { dnd }) => StatusType::Busy { dnd },
                Some(StatusTypePatch::Available {}) => StatusType::Available {},
                None => to.status,
            },
            status_text: self.status_text.unwrap_or(to.status_text),
        }
    }
}
