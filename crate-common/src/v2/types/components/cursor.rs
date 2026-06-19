// how would this work with components with multiple sets of children, eg. details/summary?

use crate::v2::types::components::{Component, ComponentId, Components};

pub struct ComponentsCursor<'a> {
    components: &'a Components,
    path: Vec<ComponentId>,
}

pub struct ComponentsCursorMut<'a> {
    components: &'a mut Components,
}

impl<'a> ComponentsCursor<'a> {
    /// get the current component
    pub fn get(&mut self) -> &'a Component {}

    /// go to the next sibling component
    pub fn next(&mut self) -> Option<&'a Component>;

    /// go to the previous sibling component
    pub fn prev(&mut self) -> Option<&'a Component>;

    /// go to the parent component
    pub fn parent(&mut self) -> Option<&'a Component>;

    /// get the zero-based index of the current component among its siblings
    pub fn index(&mut self) -> Option<usize>;

    /// get the depth of the current component in the tree
    pub fn depth(&mut self) -> Option<usize>;

    // go to the root component
    // iterate over child components
}

impl<'a> ComponentsCursorMut<'a> {
    /// remove the current component
    pub fn remove(&mut self);

    /// insert a component after the current component
    pub fn insert(&mut self);
}
