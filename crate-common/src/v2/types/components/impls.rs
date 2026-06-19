use std::{collections::HashSet, ops::Deref};

use crate::{
    v1::types::{components::IdAllocator, error::ApiResult, flume::FlumeDelta},
    v2::types::{
        MediaId,
        components::{
            Component, ComponentId,
            action::ButtonAction,
            types::{ComponentMedia, ComponentType, Components},
        },
    },
};

/// a reference to a `Component` inside a `Components`
#[derive(Debug, Clone, Copy)]
pub struct ComponentRef<'c> {
    components: &'c Components,
    component: &'c Component,
}

impl ComponentType {
    /// Whether this component type itself is interactive.
    fn is_interactive(&self) -> bool {
        match self {
            ComponentType::Button { action, .. } => {
                matches!(
                    action,
                    ButtonAction::Interaction { .. } | ButtonAction::Submit
                )
            }
            ComponentType::Input { .. }
            | ComponentType::Textarea { .. }
            | ComponentType::Select { .. }
            | ComponentType::Upload { .. }
            | ComponentType::Checkbox { .. }
            | ComponentType::Checkboxes { .. } => true,
            ComponentType::Form { .. } => true,
            _ => false,
        }
    }
}

impl Components {
    /// Get a component by its id
    pub fn get(&self, id: ComponentId) -> Option<ComponentRef<'_>> {
        self.items
            .iter()
            .find(|c| c.id == id)
            .map(|c| ComponentRef {
                components: self,
                component: c,
            })
    }

    /// Get an iterator over all components
    pub fn walk(&self) -> impl Iterator<Item = ComponentRef<'_>> {
        self.items.iter().map(|c| ComponentRef {
            components: self,
            component: c,
        })
    }

    /// Get an iterator over all root components
    pub fn children(&self) -> impl Iterator<Item = ComponentRef<'_>> {
        self.roots.iter().map(|id| self.get(*id).unwrap())
    }

    /// Whether these components are interactive.
    pub fn is_interactive(&self) -> bool {
        self.children().any(|c| c.is_interactive())
    }

    /// Delete a component by its id
    ///
    /// returns true if the component was deleted, false if the component didn't exist
    pub fn delete(&mut self, id: ComponentId) -> bool {
        if !self.items.iter().any(|c| c.id == id) {
            return false;
        }

        self.roots.retain(|r| *r != id);

        for comp in &mut self.items {
            match &mut comp.ty {
                ComponentType::Container { components, .. } => components.retain(|c| *c != id),
                ComponentType::Details {
                    summary, details, ..
                } => {
                    summary.retain(|c| *c != id);
                    details.retain(|c| *c != id);
                }
                ComponentType::Section { components, .. } => components.retain(|c| *c != id),
                ComponentType::Form { components, .. } => components.retain(|c| *c != id),
                ComponentType::Row { components, .. } => components.retain(|c| *c != id),
                _ => {}
            }
        }

        self.items.retain(|c| c.id != id);

        true
    }

    /// apply a [`FlumeDelta`] to this set of components
    pub fn patch(&mut self, delta: FlumeDelta) -> ApiResult<()> {
        todo!()
    }

    /// Append another component to this component tree.
    ///
    /// ## rules
    ///
    /// - Text can be appended to other Text (content is concatenated)
    /// - Media can be appended to Gallery (added to items)
    /// - any component can be appended to Container and Section
    /// - any component can be appended to Details. it will be appended to `details`, not `summary`.
    pub fn append(&mut self, target_id: ComponentId, other: Components) -> ApiResult<()> {
        todo!()
    }

    /// replace a component with a sequence of new ones
    pub fn replace(&mut self, target_id: ComponentId, replacements: Vec<Component>) -> bool {
        todo!()
    }

    /// minimize these components
    ///
    /// - removes any unused components
    /// - removes any unused media
    pub fn minimize(self) -> Self {
        todo!()
    }

    /// Resolve `Reference` components given the previous version of a component tree.
    pub fn resolve(self, prev: Option<Components>, media: Vec<ComponentMedia>) -> ApiResult<Self> {
        todo!()
    }

    /// Return a vec of all media ids that are referenced in these components.
    pub fn all_media_ids(&self) -> Vec<MediaId> {
        let mut ids = Vec::new();
        for comp in &self.items {
            match &comp.ty {
                ComponentType::Media { items } | ComponentType::Gallery { items } => {
                    for item in items {
                        ids.push(item.media_id);
                    }
                }
                _ => {}
            }
        }
        ids
    }

    /// Return a vec of media ids that are referenced but not in `media`.
    pub fn missing_media_ids(&self) -> Vec<MediaId> {
        let all = self.all_media_ids();
        let existing: HashSet<_> = self.media.iter().map(|m| m.id).collect();
        all.into_iter()
            .filter(|id| !existing.contains(id))
            .collect()
    }
}

impl Component {
    /// helper for [`Components::append`]
    fn append(&mut self, other: Components, id_allocator: &mut IdAllocator) -> ApiResult<()> {
        todo!()
    }

    /// clone this component, but replace all ids with new ones
    fn clone_with_new_ids(&self, id_allocator: &mut IdAllocator) -> ApiResult<Component> {
        todo!()
    }
}

impl<'c> ComponentRef<'c> {
    /// Get an iterator over this component's children
    pub fn children(&self) -> impl Iterator<Item = ComponentRef<'c>> {
        todo!();
        vec![].into_iter()
    }

    fn fold_all_children<F, B>(&self, init: B, f: F) -> B
    where
        F: Fn(B, ComponentRef<'_>) -> B,
    {
        match &self.component.ty {
            ComponentType::Container { components, .. } => components
                .iter()
                .fold(init, |i, c| f(i, self.components.get(*c).unwrap())),
            ComponentType::Section { components, .. } => components
                .iter()
                .fold(init, |i, c| f(i, self.components.get(*c).unwrap())),
            ComponentType::Form { components, .. } => components
                .iter()
                .fold(init, |i, c| f(i, self.components.get(*c).unwrap())),
            ComponentType::Row { components, .. } => components
                .iter()
                .fold(init, |i, c| f(i, self.components.get(*c).unwrap())),
            ComponentType::Details {
                summary, details, ..
            } => summary
                .iter()
                .chain(details.iter())
                .fold(init, |i, c| f(i, self.components.get(*c).unwrap())),
            _ => init,
        }
    }

    /// Whether this component or any child component is interactive.
    pub fn is_interactive(&self) -> bool {
        self.component.ty.is_interactive()
            || self.fold_all_children(false, |b, c| b || c.is_interactive())
    }
}

impl Deref for ComponentRef<'_> {
    type Target = Component;

    fn deref(&self) -> &Self::Target {
        self.component
    }
}
