#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::{Validate, ValidationErrors};

/// An email address
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[serde(transparent)]
pub struct EmailAddr(String);

#[cfg(feature = "utoipa")]
mod u {
    use utoipa::{openapi::ObjectBuilder, PartialSchema, ToSchema};

    use crate::v1::types::email::EmailAddr;

    impl ToSchema for EmailAddr {}

    impl PartialSchema for EmailAddr {
        fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
            ObjectBuilder::new()
                .schema_type(utoipa::openapi::schema::Type::String)
                .format(Some(utoipa::openapi::SchemaFormat::Custom(
                    "email".to_string(),
                )))
                .description(Some("an email address"))
                .build()
                .into()
        }
    }
}

#[cfg(feature = "validator")]
mod v {
    use validator::{Validate, ValidationError, ValidationErrors};

    use crate::v1::types::email::EmailAddr;

    impl Validate for EmailAddr {
        fn validate(&self) -> Result<(), validator::ValidationErrors> {
            if self.0.contains("@") {
                Ok(())
            } else {
                let mut v = ValidationErrors::new();
                let mut err = ValidationError::new("email");
                err.add_param("email".into(), &"must be an email address");
                v.add("email", err);
                Err(v)
            }
        }
    }
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

    // /// can see by everyone
    // pub is_public: bool,
    /// whether this is the user's primary email address
    pub is_primary: bool,
    // /// can someone with access to email can do
    // pub trust: EmailTrust,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EmailInfoPatch {
    // /// can see by everyone
    // pub is_public: Option<bool>,
    /// whether this is the user's primary email address
    ///
    /// - there can only be one primary email address
    /// - the primary address has EmailTrust::Full
    pub is_primary: Option<bool>,
    // /// can someone with access to email can do
    // pub trust: Option<EmailTrust>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
/// what someone can do with this email address
pub enum EmailTrust {
    /// can't be used for any auth
    /// the only trust levels unverified emails can have
    Never,

    /// can be used to log in
    Trusted,

    /// can be used to reset password
    /// receives security notifications
    Full,
}

impl EmailAddr {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for EmailAddr {
    type Error = ValidationErrors;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let e = EmailAddr(s);
        e.validate()?;
        Ok(e)
    }
}

impl AsRef<str> for EmailAddr {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
