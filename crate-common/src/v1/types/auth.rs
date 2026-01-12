#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;
use uuid::Uuid;

use crate::v1::types::{email::EmailAddr, util::Time, UserId};

#[cfg(feature = "validator")]
use validator::Validate;

/// response to a totp init request
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TotpInit {
    pub secret: String,
}

/// request body for totp_validate or totp_exec
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct TotpVerificationRequest {
    // FIXME: max length 6 chars
    // #[cfg_attr(feature = "utoipa", schema())]
    // #[cfg_attr(feature = "validator", validate())]
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TotpRecoveryCodes {
    pub codes: Vec<TotpRecoveryCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TotpRecoveryCode {
    pub code: String,

    /// if this is Some the code can no longer be used
    pub used_at: Option<Time>,
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
pub struct WebauthnChallenge {
    /// public key credentials request as stringified json
    pub challenge: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct WebauthnFinish {
    /// if this authenticator should be registered if it doesn't exist yet
    pub register: bool,

    /// public key credentials response as stringified json
    pub credential: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct WebauthnAuthenticator {
    pub id: Uuid,
    pub name: String,
    pub created_at: Time,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct WebauthnPatch {
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuthState {
    /// if there is at least one verified and primary email address
    ///
    /// this is used for magic links and password resets
    pub has_email: bool,

    /// if the user has registered a totp provider
    pub has_totp: bool,

    /// if a password has been set
    pub has_password: bool,

    /// the oauth providers this user has authenticated with
    pub oauth_providers: Vec<String>,

    /// registered webauthn authenticators
    pub authenticators: Vec<WebauthnAuthenticator>,
}

impl AuthState {
    /// if its technically possible for this user to login after logging out
    pub fn can_login(&self) -> bool {
        // totp ignored, it only does 2fa
        // has_password ignored, it only is effective if an email is set
        // (technically, you *can* login with user id + password, but people probably won't remember their user id)
        !self.oauth_providers.is_empty() || self.has_email || !self.authenticators.is_empty()
    }
}
