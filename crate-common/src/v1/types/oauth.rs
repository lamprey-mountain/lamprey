use core::fmt;
use lamprey_macros::record;
use std::{ops::Deref, str::FromStr};
use url::Url;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{
    ApplicationId, User, UserId,
    application::Application,
    email::EmailAddr,
    error::{ApiError, ErrorCode},
};

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

/// an oauth scope
// TODO: use strum for this (if it can handle "identify" | "openid")
// TODO: remove "implied scopes"?
#[record]
#[derive(Copy, PartialEq, Eq, Hash, strum::EnumIter)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// basic user profle information
    ///
    /// affects user_get and oauth_userinfo
    Identify,

    /// same as identify
    Openid,

    /// return email address in user profile
    ///
    /// implies `identify`
    Email,

    /// list rooms the user is in
    ///
    /// with `identify`, this returns the room member
    Rooms,

    /// list friends the user has
    ///
    /// implies `identify`
    Relationships,

    /// full read/write access to the user's account (except auth)
    ///
    /// in the future, this will be split into separate scopes
    ///
    /// implies all of the above scopes
    Full,

    /// full read/write access to /auth. implies `full`. very dangerous, will be reworked later!
    ///
    /// implies `full`
    Auth,
}

// TODO: copy macro from crate-common/src/v1/types/permission/mod.rs
bitflags::bitflags! {
    /// compact representation of a set of scopes
    ///
    /// for internal use, not sent to clients
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct ScopeBits: u8 {
        const Identify = 1 << 0;
        const Openid = 1 << 1;
        const Email = 1 << 2;
        const Rooms = 1 << 3;
        const Relationships = 1 << 4;
        const Full = 1 << 5;
        const Auth = 1 << 6;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Scopes(pub Vec<Scope>);

impl Deref for Scopes {
    type Target = Vec<Scope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for Scopes {
    type Item = Scope;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Scopes {
    type Item = &'a Scope;
    type IntoIter = std::slice::Iter<'a, Scope>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Scopes {
    /// check if this set of scopes contains a scope
    pub fn has(&self, scope: &Scope) -> bool {
        self.0.iter().any(|s| s.implies(scope))
    }

    /// check that this set of scopes contains a required scope, returning an error if it is missing
    pub fn ensure(&self, scope: &Scope) -> Result<(), ApiError> {
        if self.has(scope) {
            Ok(())
        } else {
            Err(ApiError {
                required_scopes: vec![scope.clone()],
                ..ApiError::from_code(ErrorCode::MissingScopes)
            })
        }
    }

    /// check that this set of scopes contains all required scopes, returning an error if any are missing
    pub fn ensure_all(&self, scopes: &[Scope]) -> Result<(), ApiError> {
        let mut missing = vec![];

        for required_scope in scopes {
            if !self.has(required_scope) {
                missing.push(*required_scope);
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(ApiError {
                required_scopes: missing.clone(),
                ..ApiError::from_code(ErrorCode::MissingScopes)
            })
        }
    }
}

impl Scope {
    /// check if this scope implies another scope
    pub fn implies(&self, other: &Scope) -> bool {
        if self == other {
            return true;
        }

        // NOTE: identify and openid imply each other (they're logically the same scope)
        match self {
            Scope::Auth => true,
            Scope::Full => matches!(
                other,
                Scope::Email | Scope::Identify | Scope::Rooms | Scope::Relationships
            ),
            Scope::Relationships => *other == Scope::Identify,
            Scope::Rooms => *other == Scope::Identify,
            Scope::Email => *other == Scope::Identify,
            Scope::Identify => *other == Scope::Openid,
            Scope::Openid => *other == Scope::Identify,
        }
    }

    /// get a human-readable description of the scope
    // NOTE: keep this in sync with frontend getScopeDescription
    pub fn description(&self) -> &'static str {
        match self {
            Scope::Identify | Scope::Openid => "basic profile information",
            Scope::Email => "return email address in user profile",
            Scope::Rooms => "list rooms the user is in",
            Scope::Relationships => "list friends the user has",
            Scope::Full => "full access to your account",
            Scope::Auth => "full access, including authorization information",
        }
    }
}

impl ScopeBits {
    /// check if this set of scopes contains a scope
    pub fn has(&self, scope: Scope) -> bool {
        self.contains(scope.into())
    }

    /// expand this bitset into a vec of scopes
    // TODO: serde serialize/deserialize from this
    pub fn to_vec(&self) -> Vec<Scope> {
        Scopes::from(*self).0
    }

    /// check that this set of scopes contains a required scope, returning an error if it is missing
    pub fn ensure_single(&self, scope: Scope) -> Result<(), ApiError> {
        if self.has(scope) {
            Ok(())
        } else {
            Err(ApiError {
                required_scopes: vec![scope.clone()],
                ..ApiError::from_code(ErrorCode::MissingScopes)
            })
        }
    }

    /// check that this set of scopes contains all required scopes, returning an error if any are missing
    pub fn ensure_all(&self, required: ScopeBits) -> Result<(), ApiError> {
        let missing = required - *self;

        if missing.is_empty() {
            Ok(())
        } else {
            Err(ApiError {
                required_scopes: missing.to_vec(),
                ..ApiError::from_code(ErrorCode::MissingScopes)
            })
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Scope::Identify => "identify",
            Scope::Openid => "openid",
            Scope::Email => "email",
            Scope::Rooms => "rooms",
            Scope::Relationships => "relationships",
            Scope::Full => "full",
            Scope::Auth => "auth",
        };
        f.write_str(s)
    }
}

impl FromStr for Scope {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "identify" => Ok(Scope::Identify),
            "openid" => Ok(Scope::Openid),
            "email" => Ok(Scope::Email),
            "rooms" => Ok(Scope::Rooms),
            "relationships" => Ok(Scope::Relationships),
            "full" => Ok(Scope::Full),
            "auth" => Ok(Scope::Auth),
            _ => Err(()),
        }
    }
}

impl From<Scope> for ScopeBits {
    fn from(value: Scope) -> Self {
        match value {
            Scope::Identify => ScopeBits::Identify,
            Scope::Openid => ScopeBits::Openid,
            Scope::Email => ScopeBits::Email,
            Scope::Rooms => ScopeBits::Rooms,
            Scope::Relationships => ScopeBits::Relationships,
            Scope::Full => ScopeBits::Full,
            Scope::Auth => ScopeBits::Auth,
        }
    }
}

impl From<&Scopes> for ScopeBits {
    fn from(value: &Scopes) -> Self {
        let mut bits = ScopeBits::empty();
        for scope in &value.0 {
            match scope {
                Scope::Identify => bits |= ScopeBits::Identify,
                Scope::Openid => bits |= ScopeBits::Openid,
                Scope::Email => bits |= ScopeBits::Email,
                Scope::Rooms => bits |= ScopeBits::Rooms,
                Scope::Relationships => bits |= ScopeBits::Relationships,
                Scope::Full => bits |= ScopeBits::Full,
                Scope::Auth => bits |= ScopeBits::Auth,
            }
        }
        bits
    }
}

impl From<ScopeBits> for Scopes {
    fn from(value: ScopeBits) -> Self {
        let mut scopes = Vec::new();
        if value.contains(ScopeBits::Identify) {
            scopes.push(Scope::Identify);
        }
        if value.contains(ScopeBits::Openid) {
            scopes.push(Scope::Openid);
        }
        if value.contains(ScopeBits::Email) {
            scopes.push(Scope::Email);
        }
        if value.contains(ScopeBits::Rooms) {
            scopes.push(Scope::Rooms);
        }
        if value.contains(ScopeBits::Relationships) {
            scopes.push(Scope::Relationships);
        }
        if value.contains(ScopeBits::Full) {
            scopes.push(Scope::Full);
        }
        if value.contains(ScopeBits::Auth) {
            scopes.push(Scope::Auth);
        }
        Scopes(scopes)
    }
}
