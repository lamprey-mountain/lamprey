use lamprey_macros::record;

use crate::v1::types::components::{
    ComponentCanonical, ComponentCreate, ComponentId, ComponentsCanonical, ComponentsCreate,
};
use crate::v1::types::metadata::Metadata;
use crate::v1::types::{MessageId, ParseMentions};

/// request to create a new flume
#[record]
pub struct FlumeCreate {
    /// the message this flume is replying to
    #[serde(default)]
    pub reply_id: Option<MessageId>,

    /// mentions to parse from initial components
    ///
    /// note that you can *only* mention on flume create; editing in a mention later will *not* create a notification
    #[serde(default)]
    pub mentions: ParseMentions,

    /// optional metadata
    #[serde(default)]
    pub metadata: Option<Metadata>,

    /// initial components
    pub components: ComponentsCreate,
}

// NOTE: i'd use generics preferably, but they don't work well with
// serde/utoipa, and components v2 will probably sidestep the issue altogether
// /// a delta applied to a live flume
// pub struct FlumeDelta<C: ComponentState> {}

/// a delta sent to a client to apply to a live flume
// TODO(?): add a way to update individual fields of a component without replacing it
#[record]
pub struct FlumeDeltaCanonical {
    /// initial component tree (only present in the first delta for a new flume)
    ///
    /// when present, clients should replace their entire component tree with this.
    /// subsequent deltas will then use append/replace/delete to modify it.
    #[serde(default)]
    pub init: Option<ComponentsCanonical>,

    /// append components to an existing component
    #[serde(default)]
    #[validate(length(min = 1, max = 20))]
    pub append: Vec<FlumeAppendCanonical>,

    /// replace a component with one or more components
    ///
    /// - replacing a component with children will delete the children
    /// - replacing a component with a single component will always work
    /// - replacing a component with multiple components will work if the parent has children (Root, Details, Container, Section)
    #[serde(default)]
    #[validate(length(min = 1, max = 20))]
    pub replace: Vec<FlumeReplaceCanonical>,

    /// delete some components
    #[serde(default)]
    #[validate(length(min = 1, max = 20))]
    pub delete: Vec<ComponentId>,
}

/// a delta sent from a client to apply to a live flume
// TODO(?): add a way to update individual fields of a component without replacing it
#[record]
#[derive(Default)]
pub struct FlumeDeltaCreate {
    /// initial component tree (only present in the first delta for a new flume)
    ///
    /// when present, clients should replace their entire component tree with this.
    /// subsequent deltas will then use append/replace/delete to modify it.
    #[serde(default)]
    pub init: Option<ComponentsCreate>,

    /// append components to an existing component
    #[serde(default)]
    #[validate(length(min = 1, max = 20))]
    pub append: Vec<FlumeAppendCreate>,

    /// replace a component with one or more components
    ///
    /// - replacing a component with children will delete the children
    /// - replacing a component with a single component will always work
    /// - replacing a component with multiple components will work if the parent has children (Root, Details, Container, Section)
    #[serde(default)]
    #[validate(length(min = 1, max = 20))]
    pub replace: Vec<FlumeReplaceCreate>,

    /// delete some components
    #[serde(default)]
    #[validate(length(min = 1, max = 20))]
    pub delete: Vec<ComponentId>,
}

/// append components to an existing component
#[record]
pub struct FlumeAppendCanonical {
    /// target component to append to
    pub target: ComponentId,

    /// components to append
    #[validate(length(min = 1, max = 20))]
    pub components: Vec<ComponentCanonical>,
}

/// replace a component with one or more components
#[record]
pub struct FlumeReplaceCanonical {
    /// target component to replace
    pub target: ComponentId,

    /// replacement components
    #[validate(length(min = 1, max = 20))]
    pub components: Vec<ComponentCanonical>,
}

/// append components to an existing component
#[record]
pub struct FlumeAppendCreate {
    /// target component to append to
    pub target: ComponentId,

    /// components to append
    #[validate(length(min = 1, max = 20))]
    pub components: Vec<ComponentCreate>,
}

/// replace a component with one or more components
#[record]
pub struct FlumeReplaceCreate {
    /// target component to replace
    pub target: ComponentId,

    /// replacement components
    #[validate(length(min = 1, max = 20))]
    pub components: Vec<ComponentCreate>,
}

/// current state of a flume
#[record]
#[derive(Copy, PartialEq, Eq)]
pub enum FlumeState {
    /// currently receiving updates
    Live,

    /// committed by user, no longer receiving updates
    Committed,

    /// autocommitted due to inactivity
    Autocommitted,
}

/// flume metadata for a message
#[record]
pub struct MessageFlume {
    /// current state of the flume
    pub state: FlumeState,
}
