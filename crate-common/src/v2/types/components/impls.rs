use std::{collections::HashSet, ops::Deref};

use crate::{
    v1::types::{
        components::IdAllocator,
        error::{ApiError, ApiResult, ErrorCode},
    },
    v2::types::{
        MediaId,
        components::{
            Component, ComponentId,
            components::{ComponentType, Components},
        },
        flume::FlumeDelta,
        media::MediaReference,
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
            ComponentType::Button { action, .. } => action.is_interactive(),
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

    /// whether this component is usable in an inline context
    fn is_usable_inline(&self) -> bool {
        // TODO: allow more inputs? eg. Select?
        // TODO: allow Section?
        // TODO: allow Media?
        // TODO: handle Reference and Template

        matches!(
            self,
            ComponentType::Button { .. } | ComponentType::Text { .. }
        )
    }

    /// whether this component is usable in a `Row`
    fn is_usable_in_row(&self) -> bool {
        self.is_usable_inline()
    }

    /// whether this component is only usable in a `Form`
    fn requires_form(&self) -> bool {
        // TODO: handle Reference and Template
        // TODO(?): allow Select outside of a form (discord-style)
        // TODO(?): allow checkbox{,es} and upload with similar behavior to select

        matches!(
            self,
            ComponentType::Input { .. }
                | ComponentType::Textarea { .. }
                | ComponentType::Select { .. }
                | ComponentType::Upload { .. }
                | ComponentType::Checkbox { .. }
                | ComponentType::Checkboxes { .. }
        )
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
        // 0. process init (replace entire tree)
        if let Some(init) = delta.init {
            self.media = init.media;
            self.roots = init.roots;
            self.items = init.items;
        }

        // 1. process deletes
        for id in delta.delete {
            self.delete(id);
        }

        // 2. process replacements
        for r in delta.replace {
            self.replace(r.target, r.components)?;
        }

        // 3. process appends
        for a in delta.append {
            self.append(a.target, a.components)?;
        }

        // TODO: validate

        Ok(())
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
    pub fn append(&mut self, target_id: ComponentId, other: Components) -> ApiResult<()> {
        let mut id_allocator = IdAllocator::new();
        for c in &self.items {
            id_allocator.mark_used2(c.id.0)?;
        }

        let Some(target) = self.items.iter_mut().find(|c| c.id == target_id) else {
            // TODO: better error?
            return Err(ApiError::with_message(
                ErrorCode::NotFound,
                format!("component {} not found", target_id.0),
            ));
        };

        match &mut target.ty {
            ComponentType::Text { content } => {
                if let Some(s) = other.as_text() {
                    content.push_str(s);
                } else {
                    return Err(ApiError::with_message(
                        ErrorCode::InvalidData,
                        "only Text can be appended to Text".to_owned(),
                    ));
                }
            }
            ComponentType::Gallery { items } => {
                for their_id in &other.roots {
                    if let Some(c) = other.items.iter().find(|c| c.id == *their_id) {
                        if let ComponentType::Media { item } = &c.ty {
                            items.push(item.clone());
                        } else {
                            return Err(ApiError::with_message(
                                ErrorCode::InvalidData,
                                "only Media can be appended to Gallery".to_owned(),
                            ));
                        }
                    }
                }
            }
            ComponentType::Container { .. }
            | ComponentType::Section { .. }
            | ComponentType::Details { .. }
            | ComponentType::Form { .. }
            | ComponentType::Row { .. } => {
                // PERF: don't make this O(quadratic)
                for their_id in &other.roots {
                    if let Some(c) = other.get(*their_id) {
                        let cloned = self.import(c, &mut id_allocator);
                        let target = self.items.iter_mut().find(|c| c.id == target_id).unwrap();
                        match &mut target.ty {
                            ComponentType::Container { components, .. }
                            | ComponentType::Section { components, .. }
                            | ComponentType::Details {
                                details: components,
                                ..
                            }
                            | ComponentType::Form { components, .. }
                            | ComponentType::Row { components, .. } => {
                                components.push(cloned.id);
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        todo!("error")
                    }
                }
            }
            _ => {
                return Err(ApiError::with_message(
                    ErrorCode::InvalidData,
                    "cannot append to this component type".to_owned(),
                ));
            }
        }

        Ok(())
    }

    /// import a component from another tree, ensuring ids don't conflict
    // NOTE: should this be pub?
    // NOTE: should i store IdAllocator in Components? should i implement a wrapper around Components that includes an id allocator?
    fn import(&mut self, target: ComponentRef, id_allocator: &mut IdAllocator) -> Component {
        let new_id = id_allocator.allocate(Some(target.component.id));
        let mut new_ty = target.component.ty.clone();

        let mut clone_children = |ids: &[ComponentId]| -> Vec<ComponentId> {
            let mut new_ids = Vec::with_capacity(ids.len());
            for id in ids {
                if let Some(child) = target.components.get(*id) {
                    let cloned = self.import(child, id_allocator);
                    new_ids.push(cloned.id);
                } else {
                    todo!("error handling")
                }
            }
            new_ids
        };

        match &mut new_ty {
            ComponentType::Container { components, .. }
            | ComponentType::Section { components, .. }
            | ComponentType::Form { components, .. }
            | ComponentType::Row { components, .. } => {
                *components = clone_children(&components);
            }
            ComponentType::Details {
                summary, details, ..
            } => {
                *summary = clone_children(&summary);
                *details = clone_children(&details);
            }
            _ => {}
        }

        let cloned = Component {
            id: new_id,
            ty: new_ty,
            allow: target.component.allow.clone(),
        };

        cloned
    }

    /// replace a component with a sequence of new ones
    pub fn replace(&mut self, target_id: ComponentId, replacements: Components) -> ApiResult<()> {
        let mut id_allocator = IdAllocator::new();
        for c in &self.items {
            id_allocator.mark_used2(c.id.0)?;
        }

        let mut replacement_ids = vec![];
        for their_id in &replacements.roots {
            if let Some(c) = replacements.get(*their_id) {
                let cloned = self.import(c, &mut id_allocator);
                replacement_ids.push(cloned.id);
            } else {
                todo!("error")
            }
        }

        if self.roots.contains(&target_id) {
            let pos = self.roots.iter().position(|r| *r == target_id).unwrap();
            self.roots.splice(pos..pos + 1, replacement_ids);
            return Ok(());
        }

        // TODO: add an easier method of getting parent
        for comp in &mut self.items {
            let found = match &mut comp.ty {
                ComponentType::Container { components, .. }
                | ComponentType::Section { components, .. }
                | ComponentType::Form { components, .. }
                | ComponentType::Row { components, .. } => {
                    if let Some(pos) = components.iter().position(|c| *c == target_id) {
                        components.splice(pos..pos + 1, replacement_ids.clone());
                        true
                    } else {
                        false
                    }
                }
                ComponentType::Details {
                    summary, details, ..
                } => {
                    if let Some(pos) = summary.iter().position(|c| *c == target_id) {
                        summary.splice(pos..pos + 1, replacement_ids.clone());
                        true
                    } else if let Some(pos) = details.iter().position(|c| *c == target_id) {
                        details.splice(pos..pos + 1, replacement_ids.clone());
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if found {
                return Ok(());
            }
        }

        todo!("target component not found error")
    }

    /// prune these components
    ///
    /// - remove any unused components
    /// - remove any unused media
    pub fn prune(&mut self) {
        self.prune_components();
        self.prune_media();
    }

    /// remove any unused components
    pub fn prune_components(&mut self) {
        let mut reachable_ids = HashSet::new();
        for root in &self.roots {
            self.collect_reachable_ids(*root, &mut reachable_ids);
        }

        self.items.retain(|c| reachable_ids.contains(&c.id));
    }

    /// remove any unused media
    pub fn prune_media(&mut self) {
        let ids: HashSet<MediaId> = self.referenced_media_ids().collect();
        self.media.retain(|m| ids.contains(&m.id));
    }

    fn collect_reachable_ids(&self, id: ComponentId, reachable: &mut HashSet<ComponentId>) {
        if let Some(comp_ref) = self.get(id) {
            reachable.insert(id);
            for child in comp_ref.children() {
                self.collect_reachable_ids(child.id, reachable);
            }
        } else {
            // TODO: error handling?
        }
    }

    /// compact these components
    ///
    /// - prune these components
    /// - reallocate component ids to be sequential
    pub fn compact(mut self) -> Self {
        self.prune();
        todo!("realloc component ids")
    }

    // /// Resolve `Reference` components given the previous version of a component tree.
    // pub fn resolve(self, prev: Option<Components>, media: Vec<ComponentMedia>) -> ApiResult<Self> {
    //     todo!()
    // }

    /// Return an iterator over all [`MediaReference`]s that are referenced in these components.
    pub fn referenced_media(&self) -> impl Iterator<Item = &MediaReference> {
        self.items.iter().flat_map(|comp| match &comp.ty {
            ComponentType::Media { item } => vec![&item.media_ref].into_iter(),
            ComponentType::Gallery { items } => items
                .iter()
                .map(|i| &i.media_ref)
                .collect::<Vec<_>>()
                .into_iter(),
            _ => vec![].into_iter(),
        })
    }

    /// Return an iterator over all [`MediaReference`]s that are referenced but not in `media`.
    pub fn missing_media(&self) -> impl Iterator<Item = &MediaReference> {
        let existing: HashSet<_> = self.media.iter().map(|m| m.id).collect();
        self.referenced_media().filter(move |m| match m {
            MediaReference::Media { media_id } => !existing.contains(media_id),
            _ => true,
        })
    }

    /// Return an iterator over media ids that are referenced but not in `media`.
    pub fn missing_media_ids(&self) -> impl Iterator<Item = MediaId> + '_ {
        self.missing_media().filter_map(|m| m.media_id())
    }

    /// Return an iterator over all `MediaId`s that are explicitly referenced in these components.
    pub fn referenced_media_ids(&self) -> impl Iterator<Item = MediaId> + '_ {
        self.referenced_media().filter_map(|m| m.media_id())
    }

    /// if this component is a single Text component (ie. deserialized from a single string), return the text
    pub fn as_text(&self) -> Option<&str> {
        if let Some(id) = self.roots.first() {
            if self.roots.len() == 1 {
                let c = self
                    .items
                    .iter()
                    .find(|c| c.id == *id)
                    .expect("this should be validated");
                if let ComponentType::Text { content } = &c.ty {
                    return Some(content.as_str());
                }
            }
        }

        None
    }
}

impl<'c> ComponentRef<'c> {
    /// Get an iterator over this component's children
    // TODO: maybe create a ComponentRefIter struct for this instead of collecting into a vec first
    pub fn children(&self) -> impl Iterator<Item = ComponentRef<'c>> {
        match &self.component.ty {
            ComponentType::Container { components, .. }
            | ComponentType::Section { components, .. }
            | ComponentType::Form { components, .. }
            | ComponentType::Row { components, .. } => components
                .iter()
                .map(|id| self.components.get(*id).unwrap())
                .collect::<Vec<_>>()
                .into_iter(),
            ComponentType::Details {
                summary, details, ..
            } => summary
                .iter()
                .chain(details.iter())
                .map(|id| self.components.get(*id).unwrap())
                .collect::<Vec<_>>()
                .into_iter(),
            _ => Vec::<ComponentRef<'c>>::new().into_iter(),
        }
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
