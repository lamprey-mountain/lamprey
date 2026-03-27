#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{util::Time, RoomMember, User};

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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    #[cfg_attr(feature = "serde", serde(default))]
    pub bridge: Option<Bridge>,

    /// if anyone can use this
    #[cfg_attr(feature = "serde", serde(default))]
    pub public: bool,

    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub oauth_redirect_uris: Vec<String>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub oauth_confidential: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Diff)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Connection {
    pub application: Application,
    pub scopes: Scopes,
    pub created_at: Time,
}

/// an application that is authorized to a room
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Integration {
    pub application: Application,
    pub bot: User,
    pub member: RoomMember,
}

/// where this application bridge content to
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

// TEMP: compatability
pub use super::oauth::{Scope, Scopes};
