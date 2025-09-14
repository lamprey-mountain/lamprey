use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{email::EmailAddr, UserId};

/// openid connect automatic configuration
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Autoconfig {
    pub issuer: Url,
    pub authorization_endpoint: Url,
    pub token_endpoint: Url,
    pub userinfo_endpoint: Url,
    pub scopes_supported: Vec<String>,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
}

/// user info response for openid connect
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Userinfo {
    /// user's uuid
    pub sub: UserId,

    /// primary email address (is None if email scope isnt provided)
    pub email: Option<EmailAddr>,

    /// if the provided email has been verified or not
    pub email_verified: bool,

    /// user's name
    pub name: String,

    /// html url to the user's profile page
    pub profile: String,

    /// calculated from version_id
    pub updated_at: u64,

    /// link to the user's avatar. returns the full size image, not a thumbnail.
    pub picture: Option<Url>,
}
