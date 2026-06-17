use url::Url;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::{
    v1::types::{
        ChannelId, RedexId, UserId,
        misc::{Time, hashes::Hashes},
        redex::{
            License, RedexAuthor, RedexFormat, RedexHandler, RedexOrigin, RedexPermission, Semver,
        },
    },
    v2::types::media::{Media, MediaReference},
};

/// some code that can run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Redex {
    pub id: RedexId,
    pub channel_id: ChannelId,
    pub creator_id: UserId,
    pub created_at: Time,
    pub deleted_at: Option<Time>,
    pub deploy: RedexDeployment,

    /// the capabilities that were granted to this redex
    pub permissions: Vec<RedexPermission>,

    /// detected inputs for this script
    pub handlers: Vec<RedexHandler>,
}

/// data added to a channel for redexes
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelRedex {
    pub format: RedexFormat,
    pub location: RedexLocation,
    pub metadata: RedexMetadata,

    /// the capabilities that were granted to this redex
    pub permissions: Vec<RedexPermission>,

    /// detected inputs for this script
    pub handlers: Vec<RedexHandler>,

    /// the currently active deployment of a redex
    pub deploy: RedexDeployment,
}

/// a currently live redex
///
/// currently, redexes can only have one deployment. in the future, this may change to be more flexible.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexDeployment {
    /// when this deployment was last updated
    pub updated_at: Time,

    /// currenst status of this deployment
    pub status: RedexDeploymentStatus,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RedexDeploymentStatus {
    Processing,
    Live,
    Failed { code: String, message: String },
}

/// where a redex's source is stored
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RedexLocation {
    /// stored on the host
    ///
    /// only admins can create redexes that are Local
    // maybe i can take it a step further and only allow it in the config file?
    Local { path: String },

    /// stored on the server through media
    Media { media: Media },

    /// uses a document. can only be used with javascript scripts.
    Document,
}

/// used to set a RedexLocation
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RedexLocationUpdate {
    /// stored on the host
    ///
    /// only admins can create redexes that are Local
    // maybe i can take it a step further and only allow it in the config file?
    Local { path: String },

    /// stored on the server through media
    Media {
        #[cfg_attr(feature = "serde", serde(flatten))]
        media_reference: MediaReference,
    },

    /// uses a document. can only be used with javascript scripts.
    #[default]
    Document,
}

/// request body for updating redex content
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexUpdate {
    pub format: RedexFormat,
    pub location: RedexLocationUpdate,
}

/// serialized json content of a redex
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SerializedRedex {
    pub files: Vec<SerializedRedexFile>,

    /// the recursively calculated hashes
    pub hashes: Hashes,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SerializedRedexFile {
    /// file name
    ///
    /// defaults to index.js
    pub name: String,

    /// contents of the file
    pub content: String,

    // extracted from history
    pub created_at: Time,
    pub updated_at: Time,

    /// the hashes of this file
    pub hashes: Hashes,
    // val town has type: directory | file | interval | http | email | script
    // interval | http | email are if the file has any handlers, maybe i can indicate the handlers a file has?
}

/// external code this redex requires to run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Dependency {
    pub ty: DependencyType,
    pub metadata: RedexMetadata,
    pub status: DependencyStatus,

    /// information about the latest version that can be updated
    pub update: Option<DependencyUpdate>,

    /// information about any important bugs/issues
    #[cfg(feature = "feat_redex_dependency_advisories")]
    pub advisories: Vec<DependencyAdvisory>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DependencyGraph {
    pub nodes: Vec<Dependency>,

    /// the edges of the dependency graph
    ///
    /// numbers correspond to index in `nodes`. `edge.0` depends on `edge.1`.
    pub edges: Vec<(u64, u64)>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DependencyUpdate {
    pub latest_metadata: RedexMetadata,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum DependencyStatus {
    /// downloading the dependency
    Loading,

    /// processing/loading the dependency
    Processing,

    /// dependency is valid
    Valid,

    /// failed to load the dependency
    Failed { code: String, message: String },
}

/// a request to update the dependencies of a redex
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DependencyUpdateRequest {
    /// update these dependencies
    pub nodes: Vec<u64>,

    /// whether to create a new deployment after updating
    #[cfg_attr(feature = "serde", serde(default))]
    pub redeploy: bool,
}

/// a reason to be worried about a dependency
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DependencyAdvisory {
    pub created_at: Time,
    pub updated_at: Option<Time>,

    pub summary: String,
    pub description: String,
    // TODO: urls, maybe link to cves, etc
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum DependencyType {
    /// a redex in the same channel
    Redex { redex_id: RedexId },

    /// a js/ts script fetched via http
    Http { url: Url },

    /// a npm package
    Npm { name: String, version: String },

    /// a jsr package
    Jsr { name: String, version: String },

    /// a builtin module
    // NOTE: maybe remove?
    Builtin { name: String },
}

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

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Hashes::is_empty"))]
    pub hashes: Hashes,
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
            hashes: Hashes::default(),
        }
    }
}
