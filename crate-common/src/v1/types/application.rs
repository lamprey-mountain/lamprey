use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::util::Time;

use super::{util::Diff, ApplicationId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub bridge: bool,

    /// if anyone can use this
    pub public: bool,

    /// only returned on oauth token rotate endpoint
    pub oauth_secret: Option<String>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8)))]
    pub oauth_redirect_uris: Vec<String>,

    /// oauth whether this client can keep secrets confidential
    pub oauth_confidential: bool,
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
    pub bridge: bool,

    /// if anyone can use this
    #[serde(default)]
    pub public: bool,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8)))]
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
    pub bridge: Option<bool>,

    /// if anyone can use this
    pub public: Option<bool>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8)))]
    pub oauth_redirect_uris: Option<Vec<String>>,
    pub oauth_confidential: Option<bool>,
}

/// an application that is authorized to a user
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Connection {
    pub application: Application,
    pub scopes: Vec<Scope>,
    pub created_at: Time,
}

/// an oauth scope
///
/// WORK IN PROGRESS!!! SUBJECT TO CHANGE!!!
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// basic user profle information
    ///
    /// affects user_get and oauth_userinfo
    #[serde(alias = "openid")]
    Identify,

    /// full read/write access to the user's account (except auth)
    ///
    /// in the future, this will be split into separate scopes
    Full,

    /// full read/write access to /auth. implies `full`. very dangerous, will be reworked later!
    Auth,
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
            "full" => Ok(Scope::Full),
            "auth" => Ok(Scope::Auth),
            _ => Err(()),
        }
    }
}
