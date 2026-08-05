use lamprey_macros::record;
use uuid::Uuid;

use crate::v1::types::{UserId, email::EmailAddr, util::Time};

/// response to a totp init request
#[record]
#[derive(PartialEq, Eq)]
pub struct TotpInit {
    pub secret: String,
}

/// request body for totp_validate or totp_exec
#[record]
#[derive(PartialEq, Eq)]
pub struct TotpVerificationRequest {
    /// the totp code or recovery code
    pub code: String,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct TotpRecoveryCodes {
    pub codes: Vec<TotpRecoveryCode>,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct TotpRecoveryCode {
    pub code: String,

    /// if this is Some the code can no longer be used
    pub used_at: Option<Time>,
}

/// Request body for email authentication completion
#[record]
#[derive(PartialEq, Eq)]
pub struct AuthEmailComplete {
    pub code: String,
}

// TODO(#267): look into zeroing out/erasing passwords after handling
#[record]
#[derive(PartialEq, Eq)]
pub struct PasswordSet {
    pub password: String,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct PasswordExec {
    pub password: String,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ident: PasswordExecIdent,
}

/// who's logging in
#[record]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum PasswordExecIdent {
    UserId { user_id: UserId },
    Email { email: EmailAddr },
}

#[record]
#[derive(PartialEq, Eq)]
pub struct CaptchaChallenge {
    pub code: String,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct CaptchaResponse {
    pub code: String,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct WebauthnChallenge {
    /// public key credentials request as stringified json
    pub challenge: String,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct WebauthnFinish {
    /// if this authenticator should be registered if it doesn't exist yet
    pub register: bool,

    /// public key credentials response as stringified json
    pub credential: String,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct WebauthnAuthenticator {
    pub id: Uuid,
    pub name: String,
    pub created_at: Time,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct WebauthnPatch {
    pub name: Option<String>,
}

#[record]
#[derive(PartialEq, Eq)]
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

    /// if this user is considered to have multi factor authentication enabled on their account
    pub fn has_mfa(&self) -> bool {
        // NOTE: maybe i should add a server-side configuration option for if a given oauth provider counts for mfa
        // though this will come later, since there are some tricky nuances
        self.has_totp || !self.authenticators.is_empty()
    }
}

/// Query parameters for oauth redirect callback
#[record]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct AuthOauthRedirectParams {
    pub state: String,
    pub code: String,
}
