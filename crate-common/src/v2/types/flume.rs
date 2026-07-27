use lamprey_macros::record;

use crate::v2::types::components::{ComponentId, Components};

/// a delta applied to a live flume
// TODO: add a way to update individual fields of a component without replacing it
#[record]
pub struct FlumeDelta {
    /// initial component tree (only present in the first delta for a new flume)
    ///
    /// when present, clients should replace their entire component tree with this.
    /// subsequent deltas will then use append/replace/delete to modify it.
    #[serde(default)]
    pub init: Option<Components>,

    /// append components to an existing component
    #[serde(default)]
    #[validate(length(min = 1, max = 20))]
    pub append: Vec<FlumeAppend>,

    /// replace a component with one or more components
    ///
    /// - replacing a component with children will delete the children
    /// - replacing a component with a single component will always work
    /// - replacing a component with multiple components will work if the parent has children (Root, Details, Container, Section)
    #[serde(default)]
    #[validate(length(min = 1, max = 20))]
    pub replace: Vec<FlumeReplace>,

    /// delete these components
    #[serde(default)]
    #[validate(length(min = 1, max = 20))]
    pub delete: Vec<ComponentId>,
}

/// append components to an existing component
#[record]
pub struct FlumeAppend {
    /// target component to append to
    pub target: ComponentId,

    /// components to append
    // #[validate(length(min = 1, max = 20))]
    pub components: Components,
}

/// replace a component with one or more components
#[record]
pub struct FlumeReplace {
    /// target component to replace
    pub target: ComponentId,

    /// replacement components
    // #[validate(length(min = 1, max = 20))]
    pub components: Components,
}
