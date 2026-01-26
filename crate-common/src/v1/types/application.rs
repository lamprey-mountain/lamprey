use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    error::{ApiError, ErrorCode},
    util::Time,
    RoomMember, User,
};

use super::{util::Diff, ApplicationId, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Application {
    pub id: ApplicationId,
    pub owner_id: UserId,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    /// enables managing Puppet users
    pub bridge: Option<Bridge>,

    /// if anyone can use this
    pub public: bool,

    // TODO: move oauth_foo fields below to oauth_client: ApplicationOauthClient
    /// only returned on oauth token rotate endpoint
    pub oauth_secret: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    pub oauth_redirect_uris: Vec<String>,

    /// oauth whether this client can keep secrets confidential
    pub oauth_confidential: bool,
    // do i really need all these urls properties, or can i get away with a vec?
    // url_terms_of_service: Option<Url>,
    // url_privacy_policy: Option<Url>,
    // url_help_docs: Vec<Url>,
    // url_main_site: Vec<Url>,
    // url_interactions: Vec<Url>, // webhook
    #[cfg(any())]
    /// if this is a connection that can be displayed on users' profiles
    pub connection: Option<ApplicationConnectionProvider>,

    #[cfg(any())]
    /// if this can be used to log into lamprey
    // can only be set by admins for now?
    pub oauth_provider: Option<ApplicationOauthProvider>,

    #[cfg(any())]
    /// use lamprey as an oauth provider for your application
    pub oauth_client: Option<ApplicationOauthClient>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ApplicationConnectionProvider {
    // platform_name = application.name
    // platform_description = application.description
    // platform_url: Option<String>,
    pub fields: Vec<ApplicationConnectionProviderField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ApplicationConnectionProviderField {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: ApplicationConnectionProviderFieldType,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ApplicationConnectionProviderFieldType {
    Int,
    // TODO: string, bool, time
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ApplicationOauthProvider {
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: String,
    pub token_url: String,
    pub revocation_url: String,

    /// automatically mark users as registered if they create an account or link their account with this provider
    #[cfg_attr(feature = "serde", serde(default))]
    pub autoregister: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ApplicationOauthClient {
    /// the oauth client secret
    ///
    /// only returned on oauth token rotate endpoint
    pub secret: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    pub redirect_uris: Vec<String>,

    /// whether this client can keep secrets confidential
    pub confidential: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ApplicationCreate {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    /// enables managing Puppet users
    #[serde(default)]
    pub bridge: Option<Bridge>,

    /// if anyone can use this
    #[serde(default)]
    pub public: bool,

    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    #[serde(default)]
    pub oauth_redirect_uris: Vec<String>,

    #[serde(default)]
    pub oauth_confidential: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ApplicationPatch {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<Option<String>>,

    /// enables managing Puppet users
    pub bridge: Option<Option<Bridge>>,

    /// if anyone can use this
    pub public: Option<bool>,

    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    pub oauth_redirect_uris: Option<Vec<String>>,
    pub oauth_confidential: Option<bool>,
}

/// an application that is authorized to a user
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Connection {
    pub application: Application,
    pub scopes: Scopes,
    pub created_at: Time,
}

/// an application that is authorized to a room
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Integration {
    pub application: Application,
    pub bot: User,
    pub member: RoomMember,
}

/// where this application bridge content to
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Bridge {
    /// the human readable name of the platform
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub platform_name: Option<String>,

    /// the url where this platform can be reached
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 2048))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048), url))]
    // FIXME: use Url type instead of String
    pub platform_url: Option<String>,

    /// a description of this platform
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4096)))]
    pub platform_description: Option<String>,
}

// TODO: move to oauth
/// an oauth scope
///
/// WORK IN PROGRESS!!! SUBJECT TO CHANGE!!!
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, EnumIter)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// basic user profle information
    ///
    /// affects user_get and oauth_userinfo
    #[serde(alias = "openid")]
    Identify,

    /// return email address in user profile
    ///
    /// implies `identify`
    Email,

    /// full read/write access to the user's account (except auth)
    ///
    /// in the future, this will be split into separate scopes
    ///
    /// implies `email` and `identify`
    Full,

    /// full read/write access to /auth. implies `full`. very dangerous, will be reworked later!
    ///
    /// implies `full`
    Auth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, Default)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(transparent)]
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
            Err(ApiError::from_code(ErrorCode::MissingScopes {
                scopes: Scopes(vec![scope.clone()]),
            }))
        }
    }

    /// check that this set of scopes contains all required scopes, returning an error if any are missing
    pub fn ensure_all(&self, scopes: &[Scope]) -> Result<(), ApiError> {
        let mut missing = vec![];

        for required_scope in scopes {
            if !self.has(required_scope) {
                missing.push(required_scope.clone());
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(ApiError::from_code(ErrorCode::MissingScopes {
                scopes: Scopes(missing),
            }))
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
            Scope::Full => matches!(other, Scope::Email | Scope::Identify),
            Scope::Email => *other == Scope::Identify,
            Scope::Identify => false,
        }
    }
}

impl Diff<Application> for ApplicationPatch {
    fn changes(&self, other: &Application) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.bridge.changes(&other.bridge)
            || self.public.changes(&other.public)
            || self.oauth_redirect_uris.changes(&other.oauth_redirect_uris)
            || self.oauth_confidential.changes(&other.oauth_confidential)
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Scope::Identify => "identify",
            Scope::Email => "email",
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
            "full" => Ok(Scope::Full),
            "auth" => Ok(Scope::Auth),
            _ => Err(()),
        }
    }
}
