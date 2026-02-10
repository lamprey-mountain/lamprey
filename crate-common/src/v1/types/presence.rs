#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::v1::types::util::Time;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

/// the current presence of the user
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Presence {
    pub status: Status,

    #[cfg_attr(feature = "utoipa", schema(max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    pub activities: Vec<Activity>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Status {
    /// offline or explicitly invisible
    #[default]
    Offline,

    /// connected to the service, no special status
    Online,

    /// connected but not currently active (ie. away from keyboard)
    Away,

    /// currently unavailable to chat (ie. do not disturb)
    Busy,

    /// currently available to chat
    Available,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum Activity {
    /// custom activity
    Custom {
        #[cfg_attr(feature = "utoipa", schema(max_length = 256))]
        text: String,
        clear_at: Option<Time>,
    },
}

impl Status {
    pub fn is_online(&self) -> bool {
        self != &Status::Offline
    }
}

impl Presence {
    /// construct a default online presence
    pub fn online() -> Presence {
        Presence {
            status: Status::Online,
            activities: vec![],
        }
    }

    /// construct a default offline presence
    pub fn offline() -> Presence {
        Presence {
            status: Status::Offline,
            activities: vec![],
        }
    }

    pub fn is_online(&self) -> bool {
        matches!(
            self.status,
            Status::Online | Status::Away | Status::Busy | Status::Available
        )
    }
}

#[cfg(feature = "validator")]
mod validate {
    use serde_json::json;
    use validator::{Validate, ValidateLength, ValidationError, ValidationErrors};

    use crate::v1::types::presence::Activity;

    impl Validate for Activity {
        fn validate(&self) -> Result<(), validator::ValidationErrors> {
            let mut v = ValidationErrors::new();
            match self {
                Activity::Custom { text, .. } => {
                    if text.validate_length(None, Some(256), None) {
                        Ok(())
                    } else {
                        let mut err = ValidationError::new("length");
                        err.add_param("max".into(), &json!(256));
                        v.add("text", err);
                        Err(v)
                    }
                }
            }
        }
    }
}
