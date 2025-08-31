use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{email::EmailAddr, util::Time, SessionId, UserId};

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

// TODO(#267): look into zeroing out/erasing passwords after handling
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PasswordSet {
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PasswordExec {
    pub password: String,

    #[serde(flatten)]
    pub ident: PasswordExecIdent,
}

/// who's logging in
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum PasswordExecIdent {
    UserId { user_id: UserId },
    Email { email: EmailAddr },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CaptchaChallenge {
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CaptchaResponse {
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuthState {
    /// if there is at least one verified and primary email address
    ///
    /// (this is used for magic links and password resets)
    pub has_email: bool,

    /// if local totp state is_valid
    pub has_totp: bool,

    /// if a password has been set
    pub has_password: bool,

    /// the oauth providers this user has authenticated with
    pub oauth_providers: Vec<String>,
}
