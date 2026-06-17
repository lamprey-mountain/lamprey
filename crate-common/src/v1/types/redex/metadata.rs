use url::Url;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{ChannelId, RedexId, UserId, federation::Hostname};

/// metadata about a redex
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RedexMetadata {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub description: Option<String>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub homepage_url: Option<Url>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub authors: Vec<RedexAuthor>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub version: Option<Semver>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub license: Option<License>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub origin: Option<RedexOrigin>,
}

impl RedexMetadata {
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            homepage_url: None,
            authors: vec![],
            version: None,
            license: None,
            origin: None,
        }
    }
}

/// a reference to a redex author
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RedexAuthor {
    /// human readable name
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub user: Option<RedexAuthorOrigin>,

    // FIXME: validate length
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub url: Option<Url>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RedexAuthorOrigin {
    /// the host this user is on
    pub hostname: Hostname,

    /// the id of the user on the origin host
    pub user_id: UserId,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RedexOrigin {
    pub hostname: Hostname,
    pub channel_id: ChannelId,
    pub redex_id: RedexId,
}

/// a semantic version string
///
/// matches the format `major.minor.version-extra`. major, minor, and version are numbers, and `-extra` is optional and arbitrary alphanumeric.
// TODO: validate that this is valid semver
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Semver(pub String);

/// a spdx license identifier
// TODO: validate that this actually is a spdx license
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
pub struct License(pub String);

#[cfg(feature = "utoipa")]
mod u {
    use utoipa::{PartialSchema, ToSchema, openapi::ObjectBuilder};

    use super::License;

    impl ToSchema for License {}

    impl PartialSchema for License {
        fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
            ObjectBuilder::new()
                .schema_type(utoipa::openapi::schema::Type::String)
                .min_length(Some(1))
                .max_length(Some(64))
                .build()
                .into()
        }
    }
}

#[cfg(feature = "validator")]
mod v {
    use validator::{Validate, ValidateLength, ValidationError, ValidationErrors};

    use super::License;

    impl Validate for License {
        fn validate(&self) -> Result<(), ValidationErrors> {
            if self.0.validate_length(Some(1), Some(64), None) {
                Ok(())
            } else {
                let mut errors = ValidationErrors::new();
                let mut err = ValidationError::new("length");
                err.add_param("min".into(), &1);
                err.add_param("max".into(), &64);
                errors.add("license", err);
                Err(errors)
            }
        }
    }
}
