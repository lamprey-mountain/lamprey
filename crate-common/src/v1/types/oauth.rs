use core::fmt;
use std::{ops::Deref, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::v1::types::{
    application::Application,
    email::EmailAddr,
    error::{ApiError, ErrorCode},
    ApplicationId, User, UserId,
};

/// openid connect automatic configuration
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct OauthAuthorizeInfo {
    pub application: Application,
    pub bot_user: User,
    pub auth_user: User,
    pub authorized: bool,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct OauthAuthorizeResponse {
    pub redirect_uri: Url,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct OauthTokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<Url>,
    pub client_id: Option<ApplicationId>,
    pub client_secret: Option<String>,
    pub refresh_token: Option<String>,
    pub code_verifier: Option<String>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct OauthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: String,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Scope {
    /// basic user profle information
    ///
    /// affects user_get and oauth_userinfo
    #[cfg_attr(feature = "serde", serde(alias = "openid"))]
    Identify,

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(transparent))]
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

        match self {
            Scope::Auth => true,
            Scope::Full => matches!(
                other,
                Scope::Email | Scope::Identify | Scope::Rooms | Scope::Relationships
            ),
            Scope::Relationships => *other == Scope::Identify,
            Scope::Rooms => *other == Scope::Identify,
            Scope::Email => *other == Scope::Identify,
            Scope::Identify => false,
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Scope::Identify => "identify",
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
            "identify" | "openid" => Ok(Scope::Identify),
            "email" => Ok(Scope::Email),
            "rooms" => Ok(Scope::Rooms),
            "relationships" => Ok(Scope::Relationships),
            "full" => Ok(Scope::Full),
            "auth" => Ok(Scope::Auth),
            _ => Err(()),
        }
    }
}
