use core::fmt;
use lamprey_macros::record;
use url::Url;

use crate::v1::types::{
    ApplicationId, User, UserId,
    application::Application,
    email::EmailAddr,
    error::{ApiError, ErrorCode},
};

pub mod scope;
pub use scope::*;

#[record]
pub struct OidcClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    pub nonce: Option<String>,
}

/// OAuth 2.0 Authorization Server Metadata (RFC 8414)
#[record]
pub struct AuthServerMetadata {
    pub issuer: Url,
    pub authorization_endpoint: Url,
    pub token_endpoint: Url,
    pub jwks_uri: Url,
    pub scopes_supported: Vec<String>,
    pub response_types_supported: Vec<OauthResponseType>,
    pub grant_types_supported: Vec<OauthGrantType>,
    pub token_endpoint_auth_methods_supported: Vec<OauthAuthMethod>,
    pub code_challenge_methods_supported: Vec<OauthCodeChallengeMethod>,
}

/// OpenID Connect Discovery 1.0 Provider Metadata
#[record]
pub struct OidcDiscovery {
    pub issuer: Url,
    pub authorization_endpoint: Url,
    pub token_endpoint: Url,
    pub userinfo_endpoint: Url,
    pub jwks_uri: Url,
    pub scopes_supported: Vec<String>,
    pub response_types_supported: Vec<OauthResponseType>,
    pub grant_types_supported: Vec<OauthGrantType>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<OauthAuthMethod>,
    pub claims_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<OauthCodeChallengeMethod>,
}

#[record]
#[derive(Copy, PartialEq, Eq, strum::EnumString, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum OauthAuthMethod {
    #[strum(serialize = "client_secret_post")]
    ClientSecretPost,

    #[strum(serialize = "client_secret_basic")]
    ClientSecretBasic,
}

#[record]
#[derive(Copy, PartialEq, Eq, strum::EnumString, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum OauthCodeChallengeMethod {
    #[strum(serialize = "S256")]
    S256,

    #[strum(serialize = "plain")]
    Plain,
}

/// JSON Web Key Set (RFC 7517)
#[record]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

/// JSON Web Key (RFC 7517 §4, RFC 7518 §6 for algorithm-specific params)
// TODO: remove this and use the jsonwebkey crate instead
// (jsonwebkey doesn't support utoipa ToSchema, hence why im not using it for now)
#[record]
pub struct Jwk {
    /// Key type, e.g. "RSA", "EC", "OKP"
    pub kty: String,
    /// Intended use: "sig" or "enc"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#use: Option<String>,
    /// Key operations, e.g. ["verify"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_ops: Option<Vec<String>>,
    /// Algorithm, e.g. "RS256", "ES256"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alg: Option<String>,
    /// Key ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kid: Option<String>,
    /// X.509 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x5u: Option<String>,
    /// X.509 certificate chain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x5c: Option<Vec<String>>,
    /// X.509 certificate SHA-1 thumbprint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x5t: Option<String>,
    /// X.509 certificate SHA-256 thumbprint
    #[serde(rename = "x5t#S256", skip_serializing_if = "Option::is_none")]
    pub x5t_s256: Option<String>,

    // RSA params
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub e: Option<String>,

    // EC / OKP params
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crv: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
}

/// user info response for openid connect
#[record]
pub struct Userinfo {
    /// oauth issuer
    pub iss: Url,

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
    // TODO: extra fields?
    // pub preferred_username: String,
    // pub nickname: Option<String>,
    // pub locale: String,
}

/// information about an application that can be granted access to your account
#[record]
pub struct OauthAuthorizeInfo {
    /// the application itself
    pub application: Application,

    /// if the application is a bot, this is the bot user
    pub bot_user: User,

    /// the user who requested this info
    pub auth_user: User,

    /// whether this application is already authorized
    pub authorized: bool,
}

#[record]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct OauthAuthorizeParams {
    pub response_type: OauthResponseType,
    pub client_id: ApplicationId,
    pub scope: String,
    #[allow(unused)]
    pub state: Option<String>,
    pub redirect_uri: Option<Url>,
    #[allow(unused)]
    // prompt | none, defaults to none
    pub prompt: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub nonce: Option<String>,
}

#[record]
pub struct OauthAuthorizeResponse {
    pub redirect_uri: Url,
}

#[record]
#[derive(Copy, PartialEq, Eq, strum::EnumString)]
#[serde(rename_all = "snake_case")]
pub enum OauthResponseType {
    #[strum(serialize = "code")]
    Code,
    // Token,
    // IdToken,
}

#[record]
#[derive(Copy, PartialEq, Eq, strum::EnumString)]
#[serde(rename_all = "snake_case")]
pub enum OauthGrantType {
    #[strum(serialize = "authorization_code")]
    AuthorizationCode,

    #[strum(serialize = "refresh_token")]
    RefreshToken,
    // ClientCredentials,
    // device code(?)
}

#[record]
pub struct OauthTokenRequest {
    // TODO: "You can also pass your client_id and client_secret as basic authentication with client_id as the username and client_secret as the password."
    pub grant_type: OauthGrantType,
    pub code: Option<String>,
    pub redirect_uri: Option<Url>,
    pub client_id: Option<ApplicationId>,
    pub client_secret: Option<String>,
    pub refresh_token: Option<String>,
    pub code_verifier: Option<String>,
}

#[record]
pub struct OauthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
}

#[record]
pub struct OauthIntrospectResponse {
    pub active: bool,
    pub scopes: Scopes,
    pub client_id: ApplicationId,
    /// this is specified to be "human readable", but in practice it would be
    /// simpler and more useful to return the unique id of the user
    pub username: UserId,
    pub exp: Option<u64>,
}
