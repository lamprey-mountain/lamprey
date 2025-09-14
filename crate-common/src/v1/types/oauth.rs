use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::v1::types::{
    application::{Application, Scope},
    email::EmailAddr,
    ApplicationId, User, UserId,
};

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
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct OauthAuthorizeInfo {
    pub application: Application,
    pub bot_user: User,
    pub auth_user: User,
    pub authorized: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct OauthAuthorizeParams {
    pub response_type: String,
    pub client_id: ApplicationId,
    pub scope: String,
    #[allow(unused)]
    pub state: Option<String>,
    pub redirect_uri: Option<Url>,
    #[allow(unused)]
    // prompt | none, defaults to none
    pub prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct OauthAuthorizeResponse {
    pub redirect_uri: Url,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct OauthTokenRequest {
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: Url,
    pub client_id: Option<ApplicationId>,
    pub client_secret: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct OauthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct OauthIntrospectResponse {
    pub active: bool,
    pub scopes: Vec<Scope>,
    pub client_id: ApplicationId,
    /// this is specified to be "human readable", but in practice it would be
    /// simpler and more useful to return the unique id of the user
    pub username: UserId,
    pub exp: Option<u64>,
}
