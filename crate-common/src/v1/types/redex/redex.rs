use lamprey_macros::record;

use url::Url;

use crate::v1::types::misc::Time;

use crate::v1::types::redex::metadata::RedexMetadata;
use crate::v1::types::{ChannelId, MediaId, RedexId, RedexVerId, UserId};
use crate::v2::types::media::{Media, MediaReference};

/// some code that can run
#[record]
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
#[record]
pub struct RedexHandler {
    /// unique identifier for this input
    pub id: String,

    /// human readable label
    pub label: String,

    #[serde(flatten)]
    pub ty: RedexHandlerType,

    /// the capabilities this script wants
    pub capibilities: Vec<RedexCapability>,
}

#[record]
#[serde(tag = "type")]
#[derive(PartialEq, Eq)]
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
#[record]
#[serde(tag = "type")]
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
#[record]
pub struct RedexPermission {
    pub capability: RedexCapability,
    pub grant: RedexPermissionGrant,
}

#[record]
#[derive(Default)]
pub enum RedexPermissionGrant {
    Allow,
    Deny,

    #[default]
    Prompt,
}

#[record]
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

#[record]
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
#[record]
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
// TODO: rename to RedexLanguage
#[record]
#[derive(Copy, PartialEq, Eq, strum::EnumString, strum::IntoStaticStr)]
pub enum RedexFormat {
    /// javascript via quickjs
    ///
    /// uses [rquickjs](https://lib.rs/crates/rquickjs) bindings
    // may use v8 isolates in the future
    Javascript,

    /// webassembly script (either wasm or wat)
    ///
    /// uses [wasmtime](https://lib.rs/crates/wasmtime) bindings
    Webassembly,
}

impl RedexFormat {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }
}

/// where a redex's source is stored
#[record]
#[serde(tag = "type")]
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

    /// as a document
    Document,
}

/// used to set a RedexLocation
#[record]
#[serde(tag = "type")]
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

    /// as a document
    Document,
    // note that Remote and Hosted + source_url are different
    // the first is a "live pointer" wheras the latter effectively vendors a snapshot
}

/// a redex signature
// probably use ed25519, copy federation
#[record]
pub struct RedexSignature {
    pub signature: String,
    // key, ids, etc
}

/// request body for creating a new redex
#[record]
pub struct RedexCreate {
    pub format: RedexFormat,
    pub location: RedexLocationUpdate,
}

/// request body for updating redex content
#[record]
pub struct RedexContentUpdate {
    pub format: RedexFormat,
    pub location: RedexLocationUpdate,
}

/// a single redex dependency
#[record]
pub struct RedexDependency {
    /// the redex that is being depended on
    pub script: Redex,
    // creating a redex struct for *every* file seems excessive, i probably want a way to bundle multiple files in a redex
    // maybe include version constraint?
    // maybe only return a minimal version of Redex instead of the full thing?
}

#[record]
pub struct RedexDependencyLink {
    pub dependent_id: RedexId,
    pub dependency_id: RedexId,
}

/// response body for the dependency graph
#[record]
pub struct RedexDependencyGraph {
    /// all dependencies of this redex, including transitive ones
    pub dependencies: Vec<RedexDependency>,

    /// what depends on what
    pub links: Vec<RedexDependencyLink>,
}

/// request body for updating redex dependencies
#[record]
pub struct RedexDependenciesUpdate {}

impl RedexLocation {
    pub fn media_id(&self) -> Option<MediaId> {
        match self {
            RedexLocation::Local { .. } => None,
            RedexLocation::Remote { media, .. } => Some(media.id),
            RedexLocation::Hosted { media } => Some(media.id),
            RedexLocation::Document => None,
        }
    }
}

// TODO
// export type EnvDisposition =
// 	| "template" // public + cloning the script also copies over this value
// 	| "public" // all runs can read this
// 	| "secret" // access must be requested
// 	| "opaque"; // access must be requested, code cannot read data
