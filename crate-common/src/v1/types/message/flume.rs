#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::components::{self, ComponentCreate, ComponentId, Components};
use crate::v1::types::metadata::MessageMetadata;
use crate::v1::types::{MessageId, ParseMentions};

/// request to create a new flume
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct FlumeCreate {
    /// the message this flume is replying to
    #[cfg_attr(feature = "serde", serde(default))]
    pub reply_id: Option<MessageId>,

    /// mentions to parse from initial components
    ///
    /// note that you can *only* mention on flume create; editing in a mention later will *not* create a notification
    #[cfg_attr(feature = "serde", serde(default))]
    pub mentions: ParseMentions,

    /// optional metadata
    #[cfg_attr(feature = "serde", serde(default))]
    pub metadata: Option<MessageMetadata>,

    /// initial components
    pub components: Components<components::Create>,
}

/// a delta applied to a live flume
// TODO: a way to update individual fields of a component without replacing it
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct FlumeDelta {
    /// initial component tree (only present in the first delta for a new flume)
    ///
    /// when present, clients should replace their entire component tree with this.
    /// subsequent deltas will then use append/replace/delete to modify it.
    #[cfg_attr(feature = "serde", serde(default))]
    pub init: Option<Components<components::Create>>,

    /// append components to an existing component
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 20)))]
    pub append: Vec<FlumeAppend>,

    /// replace a component with one or more components
    ///
    /// - replacing a component with children will delete the children
    /// - replacing a component with a single component will always work
    /// - replacing a component with multiple components will work if the parent has children (Root, Details, Container, Section)
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 20)))]
    pub replace: Vec<FlumeReplace>,

    /// delete some components
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 20)))]
    pub delete: Vec<ComponentId>,
}

/// append components to an existing component
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct FlumeAppend {
    /// target component to append to
    pub target: ComponentId,

    /// components to append
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 20)))]
    pub components: Vec<ComponentCreate>,
}

/// replace a component with one or more components
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct FlumeReplace {
    /// target component to replace
    pub target: ComponentId,

    /// replacement components
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 20)))]
    pub components: Vec<ComponentCreate>,
}

/// current state of a flume
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum FlumeState {
    /// currently receiving updates
    Live,

    /// committed by user, no longer receiving updates
    Committed,

    /// autocommitted due to inactivity
    Autocommitted,
}

/// flume metadata for a message
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageFlume {
    /// current state of the flume
    pub state: FlumeState,
}
