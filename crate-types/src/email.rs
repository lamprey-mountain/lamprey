use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::{Validate, ValidationErrors};

use crate::util::Time;

/// An email address
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EmailAddr {
    #[cfg_attr(feature = "validator", validate(email))]
    inner: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EmailInfo {
    #[cfg_attr(feature = "validator", validate(nested))]
    /// the email address itself
    pub email: EmailAddr,

    /// user verified they have access to the email address
    pub is_verified: bool,

    /// can see by everyone
    pub is_public: bool,

    /// whether this is the user's primary email address
    pub is_primary: bool,

    /// can someone with access to email can do
    pub trust: EmailTrust,

    /// spam prevention
    #[serde(skip)]
    pub last_verification_email_sent_at: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EmailInfoPatch {
    /// can see by everyone
    pub is_public: Option<bool>,

    /// whether this is the user's primary email address
    ///
    /// - there can only be one primary email address
    /// - the primary address has EmailTrust::Full
    pub is_primary: Option<bool>,

    /// can someone with access to email can do
    pub trust: Option<EmailTrust>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
/// what someone can do with this email address
pub enum EmailTrust {
    /// can't be used for any auth
    Never,

    /// can be used to log in
    Trusted,

    /// can be used to reset password
    /// receives security notifications
    Full,
}

impl EmailAddr {
    pub fn new(s: String) -> Result<EmailAddr, ValidationErrors> {
        let e = EmailAddr { inner: s };
        e.validate()?;
        Ok(e)
    }

    pub fn into_inner(self) -> String {
        self.inner
    }
}

impl AsRef<str> for EmailAddr {
    fn as_ref(&self) -> &str {
        &self.inner
    }
}
