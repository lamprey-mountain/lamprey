use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{util::Time, SessionId};

// #[cfg(feature = "validator")]
// use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TotpState {
    pub is_valid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TotpStateWithSecret {
    #[serde(flatten)]
    pub state: TotpState,
    pub secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TotpVerificationRequest {
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TotpRecoveryCodes {
    pub items: Vec<TotpRecoveryCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TotpRecoveryCode {
    pub code: String,

    /// if this is Some the code can no longer be used
    pub used: Option<TotpRecoveryCodeUsed>,
}

/// information about who used this code
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TotpRecoveryCodeUsed {
    pub used_at: Time,

    /// is None if session no longer exists
    pub used_by: Option<SessionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuthStatus {
    pub has_verified_email: bool,
    pub has_oauth: bool,
    pub has_totp: bool,
}
