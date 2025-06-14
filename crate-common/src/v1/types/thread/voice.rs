use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// use crate::CallId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadTypeVoicePublic {
    // probably needs some kind of limit
    pub bitrate: u64,

    // probably needs some kind of limit
    pub user_limit: u64,
    // /// override the host for the current and any future calls. if None, automatically decide for me
    // pub host_override: Option<HostOverride>,

    // /// an active call
    // pub voice: Option<VoiceCall>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadTypeVoicePrivate {}
