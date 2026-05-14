#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use url::Url;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::misc::Time;

use crate::v1::types::redex::metadata::RedexMetadata;
use crate::v1::types::{ChannelId, MediaId, RedexId, RedexVerId, UserId};
use crate::v2::types::media::{Media, MediaReference};

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
    pub latest_version: RedexVersion,
    pub status: RedexStatus,

    /// the capabilities that were granted to this redex
    pub permissions: Vec<RedexPermission>,

    /// detected inputs for this script
    pub handlers: Vec<RedexHandler>,
    // TODO: pub signatures: Vec<ScriptSignature>,
    // TODO: autoupdate info: fetch error, error count, retry update at
}

/// the valid inputs to this script
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexHandler {
    /// unique identifier for this input
    pub id: String,

    /// human readable label
    pub label: String,

    #[cfg_attr(feature = "serde", serde(rename = "type", flatten))]
    pub ty: RedexHandlerType,

    /// the capabilities this script wants
    pub capibilities: Vec<RedexCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RedexHandlerType {
    /// a manual trigger/button
    Manual,

    /// an http request
    Http {
        // TODO: configurable endpoints. for now, run_id.suffix is used.
        // /// the domain name requests should go to
        // endpoint: String,
    },

    /// an api event (MessageSync)
    Event,
}

/// a capability this script requires
///
/// can also be viewed as an effect that running this script may cause
///
/// logging is considered pure
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RedexCapability {
    /// can spawn new runs
    RunSpawn,

    /// can manage all runs
    RunManage,

    /// can do http requests over the network
    Http {
        /// the hosts to allow http requests to
        ///
        /// if None, allow requests to all hosts
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        allow: Option<Vec<String>>,
    },

    /// can store things in persistent storage
    Storage,

    /// can access environment secrets
    Secrets {
        /// the secrets to allow access to
        ///
        /// if None, allow requests to all secrets
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        allow: Option<Vec<String>>,
    },
}

/// a permission granted to this redex
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexPermission {
    pub capability: RedexCapability,
    pub grant: RedexPermissionGrant,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RedexPermissionGrant {
    Allow,
    Deny,

    #[default]
    Prompt,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RedexStatus {
    /// this redex has no content
    Empty,

    /// this redex is being processed/validated for the first time
    Creating,

    /// this redex is being processed and validated
    ///
    /// old versions of the redex *may* be used while processing
    Processing,

    /// this redex is runnable
    Valid,

    /// this redex is invalid
    // TODO: add a way to find out why its invalid
    Invalid,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RedexVersionStatus {
    /// this redex version is being processed and validated
    Processing,

    /// this redex version is runnable
    Valid,

    /// this redex version is invalid
    // TODO: add a way to find out why its invalid
    Invalid,
}

/// information about a redex version
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexVersion {
    pub version_id: RedexVerId,
    pub created_at: Time,
    pub deleted_at: Option<Time>,
    pub format: RedexFormat,
    pub location: RedexLocation,
    pub metadata: RedexMetadata,
    pub status: RedexVersionStatus,
}

/// the format of a redex
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RedexFormat {
    /// javascript via quickjs
    ///
    /// uses [rquickjs](https://lib.rs/crates/rquickjs) bindings
    // may use v8 isolates in the future
    Javascript,

    /// webassembly script (either wasm or wat)
    ///
    /// probably will use wasmtime or something
    Webassembly,
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

    /// stored on a remote url
    Remote {
        media: Media,

        // same as media source_url?
        url: Url,
    },

    /// stored on the server
    Hosted { media: Media },
    // TODO: document
}

/// used to set a RedexLocation
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RedexLocationUpdate {
    /// stored on the host
    ///
    /// only admins can create redexes that are Local
    // maybe i can take it a step further and only allow it in the config file?
    Local { path: String },

    /// stored on a remote url
    Remote { url: Url },

    /// stored on the server
    Hosted {
        #[cfg_attr(feature = "serde", serde(flatten))]
        media_reference: MediaReference,
    },
    // note that Remote and Hosted + source_url are different
    // the first is a "live pointer" wheras the latter effectively vendors a snapshot
}

/// a redex signature
// probably use ed25519, copy federation
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexSignature {
    pub signature: String,
    // key, ids, etc
}

/// request body for creating a new redex
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(validator::Validate))]
pub struct RedexCreate {
    pub format: RedexFormat,
    pub location: RedexLocationUpdate,
}

/// request body for updating redex content
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexContentUpdate {
    pub format: RedexFormat,
    pub location: RedexLocationUpdate,
}

/// a single redex dependency
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexDependency {
    /// the redex that is being depended on
    pub script: Redex,
    // creating a redex struct for *every* file seems excessive, i probably want a way to bundle multiple files in a redex
    // maybe include version constraint?
    // maybe only return a minimal version of Redex instead of the full thing?
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexDependencyLink {
    pub dependent_id: RedexId,
    pub dependency_id: RedexId,
}

/// response body for the dependency graph
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexDependencyGraph {
    /// all dependencies of this redex, including transitive ones
    pub dependencies: Vec<RedexDependency>,

    /// what depends on what
    pub links: Vec<RedexDependencyLink>,
}

/// request body for updating redex dependencies
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RedexDependenciesUpdate {}

impl RedexLocation {
    pub fn media_id(&self) -> Option<MediaId> {
        match self {
            RedexLocation::Local { .. } => None,
            RedexLocation::Remote { media, .. } => Some(media.id),
            RedexLocation::Hosted { media } => Some(media.id),
        }
    }
}

// TODO
// export type EnvDisposition =
// 	| "template" // public + cloning the script also copies over this value
// 	| "public" // all runs can read this
// 	| "secret" // access must be requested
// 	| "opaque"; // access must be requested, code cannot read data
