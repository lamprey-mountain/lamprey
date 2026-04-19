#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use url::Url;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::misc::Time;

use crate::v1::types::{ScriptId, ScriptVerId, UserId};
use crate::v2::types::media::{Media, MediaReference};

/// a script that can run
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Script {
    pub id: ScriptId,
    pub creator_id: UserId,
    pub created_at: Time,
    pub deleted_at: Option<Time>,
    pub trust: ScriptTrust,

    pub latest_version: ScriptVersion,

    // TODO: being able to set what permissions are available to a script
    // though what perms are available may change with what its purpose is
    pub permissions: (),
    // autoupdate info: fetch error, error count, retry update at
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
}

/// the format of a script
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ScriptFormat {
    /// javascript via quickjs
    ///
    /// uses [rquickjs](https://lib.rs/crates/rquickjs) bindings
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
#[derive(Debug, Clone)]
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

/// trust level for a script
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ScriptTrust {
    /// this script isn't trusted at all
    Untrusted,

    /// the server doesnt trust this script but the person whos running it does
    Restricted,

    /// this script is trusted because it's local
    Local,

    /// this script is trusted because its signed
    Signed { signature: ScriptSignature },
}

/// a script signature
// probably use ed25519, copy federation
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ScriptSignature(pub String);

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
