#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use url::Url;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::misc::Time;

use crate::v1::types::{ChannelId, MediaId, ScriptId, ScriptVerId, UserId};
use crate::v2::types::media::{Media, MediaReference};

/// a script that can run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Script {
    pub id: ScriptId,
    pub channel_id: ChannelId,
    pub creator_id: UserId,
    pub created_at: Time,
    pub deleted_at: Option<Time>,
    pub latest_version: ScriptVersion,
    pub status: ScriptStatus,

    /// the effects that this script is allowed to run
    pub permissions: Vec<ScriptPermission>,

    /// detected inputs for this script
    pub inputs: Vec<ScriptInput>,
    // TODO: pub signatures: Vec<ScriptSignature>,
    // TODO: autoupdate info: fetch error, error count, retry update at
}

/// the valid inputs to this script
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ScriptInput {
    /// unique identifier for this input
    pub id: String,

    /// human readable label
    pub label: String,

    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: ScriptInputType,

    /// the {side effects, capabilities, outputs} of this script
    pub effects: Vec<ScriptEffect>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ScriptInputType {
    /// a manual trigger/button
    Manual,
    // /// an http request
    // Http {
    //     /// the domain name requests should go to
    //     endpoint: String,
    // },
}

/// a capability this script requires
///
/// can also be viewed as an effect that running this script may cause
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ScriptEffect {
    // logging is considered pure for now

    // /// an http response
    // Http,
}

// TODO: validate that ScriptInput has a valid ScriptEffect

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ScriptPermission {
    pub effect: ScriptEffect,

    /// whether this should be allowed or denied
    pub grant: ScriptPermissionGrant,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ScriptPermissionGrant {
    Allow,
    Deny,

    #[default]
    Prompt,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ScriptStatus {
    /// this script has no content
    Empty,

    /// this script is being processed/validated for the first time
    Creating,

    /// this script is being processed and validated
    ///
    /// old versions of the script *may* be used while processing
    Processing,

    /// this script is runnable
    Valid,

    /// this script is invalid
    // TODO: add a way to find out why its invalid
    Invalid,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ScriptVersionStatus {
    /// this script version is being processed and validated
    Processing,

    /// this script version is runnable
    Valid,

    /// this script version is invalid
    // TODO: add a way to find out why its invalid
    Invalid,
}

/// information about a script version
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ScriptVersion {
    pub version_id: ScriptVerId,
    pub created_at: Time,
    pub deleted_at: Option<Time>,
    pub format: ScriptFormat,
    pub location: ScriptLocation,
    pub metadata: ScriptMetadata,
    pub status: ScriptVersionStatus,
}

/// the format of a script
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ScriptFormat {
    /// javascript via quickjs
    ///
    /// uses [rquickjs](https://lib.rs/crates/rquickjs) bindings
    // may use v8 isolates in the future
    Javascript,

    /// webassembly script
    ///
    /// probably will use wasmtime or something
    Webassembly,
}

/// where a script is stored
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ScriptLocation {
    /// stored on the host
    ///
    /// only admins can create scripts that are Local
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
}

/// used to set a ScriptLocation
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ScriptLocationSet {
    /// stored on the host
    ///
    /// only admins can create scripts that are Local
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

/// metadata about a script
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ScriptMetadata {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    pub homepage_url: String,
    pub authors: Vec<String>,
    pub version: String,

    /// a spdx license identifier
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub license: String,
}

/// a script signature
// probably use ed25519, copy federation
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ScriptSignature {
    pub signature: String,
    // key, ids, etc
}

/// request body for creating a new script
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(validator::Validate))]
pub struct ScriptCreate {
    // metadata is extracted via userscript-like comments
    pub format: ScriptFormat,
    pub location: ScriptLocationSet,
}

/// request body for updating script content
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ScriptContentUpdate {
    pub format: ScriptFormat,
    pub location: ScriptLocationSet,
}

/// a single script dependency
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ScriptDependency {
    /// the script that is being depended on
    pub script: Script,
    // creating a script struct for *every* file seems excessive, i probably want a way to bundle multiple files in a script
    // maybe include version constraint?
    // maybe only return a minimal version of Script instead of the full thing?
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ScriptDependencyLink {
    pub dependent_id: ScriptId,
    pub dependency_id: ScriptId,
}

/// response body for the dependency graph
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ScriptDependencyGraph {
    /// all dependencies of this script, including transitive ones
    pub dependencies: Vec<ScriptDependency>,

    /// what depends on what
    pub links: Vec<ScriptDependencyLink>,
}

/// request body for updating script dependencies
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ScriptDependenciesUpdate {}

impl ScriptLocation {
    pub fn media_id(&self) -> Option<MediaId> {
        match self {
            ScriptLocation::Local { .. } => None,
            ScriptLocation::Remote { media, .. } => Some(media.id),
            ScriptLocation::Hosted { media } => Some(media.id),
        }
    }
}
