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

// #[derive(Debug, Clone, Copy)]
// pub struct ComponentsFoo<'c> {
//     components: &'c Components,
//     // PERF: store lookup maps for components
//     // - component id -> component
//     // - component custom id -> component
//     // - media id -> option(?) media vec index, component id(s?)
// }

/// a reference to a `Component` inside a `Components`
#[derive(Debug, Clone, Copy)]
pub struct ComponentRef<'c> {
    components: &'c Components,
    component: &'c Component,
    // maybe store a list of parents? and add fns to get
    // path: Vec<ComponentId>,
}

impl ComponentType {
    /// Whether this component type itself is interactive.
    fn is_interactive(&self) -> bool {
        match self {
            ComponentType::Button(button) => button.action.is_interactive(),
            ComponentType::Input(_)
            | ComponentType::Textarea(_)
            | ComponentType::Select(_)
            | ComponentType::Upload(_)
            | ComponentType::Checkbox(_)
            | ComponentType::Checkboxes(_) => true,
            ComponentType::Form(_) => true,
            _ => false,
        }
    }

    /// whether this component is usable in an inline context
    fn is_usable_inline(&self) -> bool {
        // TODO: allow more inputs? eg. Select?
        // TODO: allow Section?
        // TODO: allow Media?
        // TODO: handle Reference and Template

        matches!(self, ComponentType::Button(_) | ComponentType::Text(_))
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
            ComponentType::Input(_)
                | ComponentType::Textarea(_)
                | ComponentType::Select(_)
                | ComponentType::Upload(_)
                | ComponentType::Checkbox(_)
                | ComponentType::Checkboxes(_)
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
                ComponentType::Container(container) => container.components.retain(|c| *c != id),
                ComponentType::Details(details) => {
                    details.summary.retain(|c| *c != id);
                    details.details.retain(|c| *c != id);
                }
                ComponentType::Section(section) => section.components.retain(|c| *c != id),
                ComponentType::Form(form) => form.components.retain(|c| *c != id),
                ComponentType::Row(row) => row.components.retain(|c| *c != id),
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
            id_allocator.mark_used(c.id.0)?;
        }

        let Some(target) = self.items.iter_mut().find(|c| c.id == target_id) else {
            // TODO: better error?
            return Err(ApiError::with_message(
                ErrorCode::NotFound,
                format!("component {} not found", target_id.0),
            ));
        };

        match &mut target.ty {
            ComponentType::Text(text) => {
                if let Some(s) = other.as_text() {
                    text.content.push_str(s);
                } else {
                    return Err(ApiError::with_message(
                        ErrorCode::InvalidData,
                        "only Text can be appended to Text".to_owned(),
                    ));
                }
            }
            ComponentType::Gallery(gallery) => {
                for their_id in &other.roots {
                    if let Some(c) = other.items.iter().find(|c| c.id == *their_id) {
                        if let ComponentType::Media(media) = &c.ty {
                            gallery.items.push(media.item.clone());
                        } else {
                            return Err(ApiError::with_message(
                                ErrorCode::InvalidData,
                                "only Media can be appended to Gallery".to_owned(),
                            ));
                        }
                    }
                }
            }
            ComponentType::Container(_)
            | ComponentType::Section(_)
            | ComponentType::Details(_)
            | ComponentType::Form(_)
            | ComponentType::Row(_) => {
                // PERF: don't make this O(quadratic)
                for their_id in &other.roots {
                    if let Some(c) = other.get(*their_id) {
                        let cloned = self.import(c, &mut id_allocator);
                        let target = self.items.iter_mut().find(|c| c.id == target_id).unwrap();
                        match &mut target.ty {
                            ComponentType::Container(container) => {
                                container.components.push(cloned.id);
                            }
                            ComponentType::Section(section) => {
                                section.components.push(cloned.id);
                            }
                            ComponentType::Details(details) => {
                                details.details.push(cloned.id);
                            }
                            ComponentType::Form(form) => {
                                form.components.push(cloned.id);
                            }
                            ComponentType::Row(row) => {
                                row.components.push(cloned.id);
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
            ComponentType::Container(container) => {
                container.components = clone_children(&container.components);
            }
            ComponentType::Section(section) => {
                section.components = clone_children(&section.components);
            }
            ComponentType::Form(form) => {
                form.components = clone_children(&form.components);
            }
            ComponentType::Row(row) => {
                row.components = clone_children(&row.components);
            }
            ComponentType::Details(details) => {
                details.summary = clone_children(&details.summary);
                details.details = clone_children(&details.details);
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
            id_allocator.mark_used(c.id.0)?;
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
                ComponentType::Container(container) => {
                    if let Some(pos) = container.components.iter().position(|c| *c == target_id) {
                        container
                            .components
                            .splice(pos..pos + 1, replacement_ids.clone());
                        true
                    } else {
                        false
                    }
                }
                ComponentType::Section(section) => {
                    if let Some(pos) = section.components.iter().position(|c| *c == target_id) {
                        section
                            .components
                            .splice(pos..pos + 1, replacement_ids.clone());
                        true
                    } else {
                        false
                    }
                }
                ComponentType::Form(form) => {
                    if let Some(pos) = form.components.iter().position(|c| *c == target_id) {
                        form.components
                            .splice(pos..pos + 1, replacement_ids.clone());
                        true
                    } else {
                        false
                    }
                }
                ComponentType::Row(row) => {
                    if let Some(pos) = row.components.iter().position(|c| *c == target_id) {
                        row.components.splice(pos..pos + 1, replacement_ids.clone());
                        true
                    } else {
                        false
                    }
                }
                ComponentType::Details(details) => {
                    if let Some(pos) = details.summary.iter().position(|c| *c == target_id) {
                        details
                            .summary
                            .splice(pos..pos + 1, replacement_ids.clone());
                        true
                    } else if let Some(pos) = details.details.iter().position(|c| *c == target_id) {
                        details
                            .details
                            .splice(pos..pos + 1, replacement_ids.clone());
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
            ComponentType::Media(media) => vec![&media.item.media_ref].into_iter(),
            ComponentType::Gallery(gallery) => gallery
                .items
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
                if let ComponentType::Text(text) = &c.ty {
                    return Some(text.content.as_str());
                }
            }
        }

        None
    }
}

// TODO: fn walk() for ComponentRef
impl<'c> ComponentRef<'c> {
    /// Get an iterator over this component's children
    // TODO: maybe create a ComponentRefIter struct for this instead of collecting into a vec first
    pub fn children(&self) -> impl Iterator<Item = ComponentRef<'c>> {
        match &self.component.ty {
            ComponentType::Container(container) => container
                .components
                .iter()
                .map(|id| self.components.get(*id).unwrap())
                .collect::<Vec<_>>()
                .into_iter(),
            ComponentType::Section(section) => section
                .components
                .iter()
                .map(|id| self.components.get(*id).unwrap())
                .collect::<Vec<_>>()
                .into_iter(),
            ComponentType::Form(form) => form
                .components
                .iter()
                .map(|id| self.components.get(*id).unwrap())
                .collect::<Vec<_>>()
                .into_iter(),
            ComponentType::Row(row) => row
                .components
                .iter()
                .map(|id| self.components.get(*id).unwrap())
                .collect::<Vec<_>>()
                .into_iter(),
            ComponentType::Details(details) => details
                .summary
                .iter()
                .chain(details.details.iter())
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
            ComponentType::Container(container) => container
                .components
                .iter()
                .fold(init, |i, c| f(i, self.components.get(*c).unwrap())),
            ComponentType::Section(section) => section
                .components
                .iter()
                .fold(init, |i, c| f(i, self.components.get(*c).unwrap())),
            ComponentType::Form(form) => form
                .components
                .iter()
                .fold(init, |i, c| f(i, self.components.get(*c).unwrap())),
            ComponentType::Row(row) => row
                .components
                .iter()
                .fold(init, |i, c| f(i, self.components.get(*c).unwrap())),
            ComponentType::Details(details) => details
                .summary
                .iter()
                .chain(details.details.iter())
                .fold(init, |i, c| f(i, self.components.get(*c).unwrap())),
            _ => init,
        }
    }

    /// Whether this component or any child component is interactive.
    pub fn is_interactive(&self) -> bool {
        self.component.ty.is_interactive()
            || self.fold_all_children(false, |b, c| b || c.is_interactive())
    }

    // TODO: add more navigation options?
    // /// go to the next sibling component
    // pub fn next(&mut self) -> Option<ComponentRef<'c>>;
    //
    // /// go to the previous sibling component
    // pub fn prev(&mut self) -> Option<ComponentRef<'c>>;
    //
    // /// go to the parent component
    // pub fn parent(&mut self) -> Option<ComponentRef<'c>>;
    //
    // /// get the zero-based index of the current component among its siblings
    // pub fn index(&mut self) -> Option<usize>;
    //
    // /// get the depth of the current component in the tree
    // pub fn depth(&mut self) -> Option<usize>;
}

impl Deref for ComponentRef<'_> {
    type Target = Component;

    fn deref(&self) -> &Self::Target {
        self.component
    }
}
