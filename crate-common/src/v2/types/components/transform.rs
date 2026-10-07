//! logic for building and applying transformations to components

use crate::{
    v1::types::error::ApiResult,
    v2::types::components::{ComponentId, Components},
};

impl Components {
    /// create a new transform for this components
    pub fn mutate(&self) -> Transform<'_> {
        todo!()
    }

    /// apply a transformation to this set of components
    pub fn apply(mut self, t: Transform<'_>) -> ApiResult<Self> {
        todo!()
    }
}

/// a transformation that can be applied to some components
pub struct Transform<'a> {
    // do i need this field?
    components: &'a Components,
}

impl Transform<'_> {
    /// Delete a component by its id
    ///
    /// returns true if the component was deleted, false if the component didn't exist
    pub fn delete(&mut self, id: ComponentId) -> bool {
        todo!()
    }

    /// replace a component with a sequence of new ones
    pub fn replace(&mut self, target_id: ComponentId, replacements: Components) {
        todo!()
    }

    /// Append another component to this component tree.
    ///
    /// ## rules
    ///
    /// - `Text` can be appended to other `Text` (content is concatenated)
    /// - `Media` can be appended to `Gallery` (added to items)
    /// - any component can be appended to `Container` and `Section`
    /// - any component can be appended to `Details`. it will be appended to `details`, not `summary`.
    /// - valid components can be appended to `Row`
    pub fn append(&mut self, target_id: ComponentId, other: Components) {
        todo!()
    }

    /// merge another transform after this one
    pub fn merge(&mut self, other: Transform<'_>) {
        todo!()
    }

    // pub fn to_delta(&self) -> FlumeDelta;
    // pub fn from_delta(delta: FlumeDelta) -> Self;

    // add a way to resolve template/reference
}

// maybe let replace/append accept ComponentRef, Component, Components
// pub trait IntoComponents {}

// #[record]
// pub struct ComponentDeltaCreate {
//     // ...
// }

// #[record]
// pub struct ComponentDelta {
//     pub components: Vec<Component>,

//     pub init: Option<Components>,
//     pub append: Vec<ComponentAppend>,
//     pub replace: Vec<ComponentReplace>,
//     pub delete: Vec<ComponentId>,
// }

// pub struct ComponentAppend {
//     pub target: ComponentId,
//     pub component_ids: Vec<ComponentId>,
//     // pub where: String, // summary
// }

// pub struct ComponentReplace {
//     pub target: ComponentId,
//     pub component_ids: Vec<ComponentId>,
// }

// /// an error that occured while applying a delta to some components
// #[derive(Debug, thiserror::Error)]
// pub enum ComponentDeltaError {
//     // etc...
// }
