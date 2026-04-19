use super::types::*;
use crate::{
    v1::types::{
        components::{IdAllocator, ValidationState},
        error::{ApiError, ErrorCode},
        flume::FlumeDelta,
        MediaId,
    },
    v2::types::media::{Media, MediaReference},
};

impl<C: ComponentState> Components<C> {
    pub fn validate(&self) -> Result<(), ApiError> {
        let mut state = ValidationState::new();
        state.validate_count(self.inner.len(), 1, 20, "components");

        for (i, c) in self.inner.iter().enumerate() {
            state.enter_index(i, |s| c.ty.validate_inner(s));
        }

        if state.errors.is_empty() {
            Ok(())
        } else {
            Err(ApiError {
                fields: state.errors,
                ..ApiError::with_message(
                    ErrorCode::InvalidData,
                    "invalid component data".to_owned(),
                )
            })
        }
    }
}

impl<C: ComponentState> ComponentType<C> {
    /// Whether this component or any child component is interactive.
    pub fn is_interactive(&self) -> bool {
        match self {
            ComponentType::Button { .. } => true,
            ComponentType::LinkButton { .. } => false,
            ComponentType::Container { components, .. }
            | ComponentType::Section { components, .. } => {
                components.iter().any(|c| c.ty().is_interactive())
            }
            ComponentType::Details {
                summary, details, ..
            } => {
                summary.iter().any(|c| c.ty().is_interactive())
                    || details.iter().any(|c| c.ty().is_interactive())
            }
            ComponentType::Text { .. }
            | ComponentType::Reference { .. }
            | ComponentType::Media { .. }
            | ComponentType::Gallery { .. } => false,
        }
    }

    pub fn validate(&self) -> Result<(), ApiError> {
        let mut state = ValidationState::new();
        self.validate_inner(&mut state);
        if state.errors.is_empty() {
            Ok(())
        } else {
            Err(ApiError {
                fields: state.errors,
                ..ApiError::with_message(
                    ErrorCode::InvalidData,
                    "invalid component data".to_owned(),
                )
            })
        }
    }

    fn validate_inner(&self, state: &mut ValidationState) {
        state.check_component();

        match self {
            ComponentType::Button {
                label, custom_id, ..
            } => {
                state.validate_length(&custom_id.0, 1, 128, "custom_id");
                state.add_text(custom_id.0.len());
                state.validate_length(label, 1, 256, "label");
                state.add_text(label.len());
            }
            ComponentType::LinkButton { label, url, .. } => {
                state.validate_length(label, 1, 256, "label");
                state.add_text(label.len());
                if let Some(u) = url {
                    state.add_text(u.as_str().len());
                }
            }
            ComponentType::Container { components, .. } => {
                state.validate_count(components.len(), 1, 20, "container components");
                for (i, c) in components.iter().enumerate() {
                    state.enter_index(i, |s| c.ty().validate_inner(s));
                }
            }
            ComponentType::Section { components, .. } => {
                state.validate_count(components.len(), 1, 20, "section components");
                for (i, c) in components.iter().enumerate() {
                    state.enter_index(i, |s| c.ty().validate_inner(s));
                }
            }
            ComponentType::Text { content } => {
                state.validate_length(content, 1, 8192, "text");
                state.add_text(content.len());
            }
            ComponentType::Details {
                summary, details, ..
            } => {
                state.validate_count(summary.len(), 1, 20, "summary components");
                state.enter("summary", |s| {
                    for (i, c) in summary.iter().enumerate() {
                        s.enter_index(i, |s2| c.ty().validate_inner(s2));
                    }
                });

                state.validate_count(details.len(), 1, 20, "details components");
                state.enter("details", |s| {
                    for (i, c) in details.iter().enumerate() {
                        s.enter_index(i, |s2| c.ty().validate_inner(s2));
                    }
                });
            }
            ComponentType::Media { items } | ComponentType::Gallery { items } => {
                let label = if matches!(self, ComponentType::Media { .. }) {
                    "media"
                } else {
                    "gallery"
                };
                state.validate_count(items.len(), 1, 20, &format!("{label} items"));
                for (i, item) in items.iter().enumerate() {
                    state.enter_index(i, |s| {
                        if let Some(desc) = &item.description {
                            s.validate_length(desc, 1, 1024, "description");
                            s.add_text(desc.len());
                        }
                    });
                }
            }
            ComponentType::Reference { .. } => {}
        }
    }
}

impl Component<Thin> {
    /// Append another component to this component tree.
    ///
    /// ## rules
    ///
    /// - Text can be appended to other Text (content is concatenated)
    /// - Media can be appended to Gallery (added to items)
    /// - any component can be appended to Container and Section
    /// - any component can be appended to Details. it will be appended to `details`, not `summary`.
    pub fn append(
        &mut self,
        other: Component<Create>,
        id_allocator: &mut IdAllocator,
        resolve_media: impl Fn(MediaReference) -> Result<MediaId, ApiError>,
    ) -> Result<(), ApiError> {
        let other_thin = other.parse_thin_inner(None, id_allocator, &resolve_media)?;

        match &mut self.ty {
            ComponentType::Text { content } => {
                if let ComponentType::Text {
                    content: other_content,
                } = other_thin.ty
                {
                    content.push_str(&other_content);
                } else {
                    return Err(ApiError::with_message(
                        ErrorCode::InvalidData,
                        "only Text can be appended to Text".to_owned(),
                    ));
                }
            }
            ComponentType::Gallery { items } => {
                if let ComponentType::Media { items: other_items } = other_thin.ty {
                    items.extend(other_items);
                } else {
                    return Err(ApiError::with_message(
                        ErrorCode::InvalidData,
                        "only Media can be appended to Gallery".to_owned(),
                    ));
                }
            }
            ComponentType::Container { components, .. } => {
                components.push(other_thin);
            }
            ComponentType::Section { components, .. } => {
                components.push(other_thin);
            }
            ComponentType::Details { details, .. } => {
                details.push(other_thin);
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
}

impl Components<Create> {
    /// parse a Blueprint into a Canonical component
    ///
    /// if there is a previous version of a component tree, it is used
    /// to resolve `Reference` components. resolve_media should resolve a
    /// MediaReference into Media, use get_media_references first to process
    /// media
    pub fn parse<R>(
        self,
        prev: Option<&Components<Canonical>>,
        resolve_media: &R,
    ) -> Result<Components<Canonical>, ApiError>
    where
        R: Fn(MediaReference) -> Result<Media, ApiError>,
    {
        let mut id_allocator = IdAllocator::new();

        // mark ids in old tree as used
        if let Some(prev_tree) = prev {
            for component in &prev_tree.inner {
                component.visit_ids(&mut |id| id_allocator.mark_used(id.0));
            }
        }

        let mut parsed = Vec::with_capacity(self.inner.len());
        for c in self.inner {
            parsed.push(c.parse_inner(prev, &mut id_allocator, resolve_media)?);
        }

        let tree = Components { inner: parsed };
        tree.validate()?;
        Ok(tree)
    }

    /// parse a Blueprint into a Thin component
    ///
    /// if there is a previous version of a component tree, it is used
    /// to resolve `Reference` components. resolve_media should resolve a
    /// MediaReference into Media, use get_media_references first to process
    /// media
    pub fn parse_thin<R>(
        self,
        prev: Option<&Components<Thin>>,
        resolve_media: &R,
    ) -> Result<Components<Thin>, ApiError>
    where
        R: Fn(MediaReference) -> Result<MediaId, ApiError>,
    {
        let mut id_allocator = IdAllocator::new();

        // mark ids in old tree as used
        if let Some(prev_tree) = prev {
            for component in &prev_tree.inner {
                component.visit_ids(&mut |id| id_allocator.mark_used(id.0));
            }
        }

        let mut parsed = Vec::with_capacity(self.inner.len());
        for c in self.inner {
            parsed.push(c.parse_thin_inner(prev, &mut id_allocator, resolve_media)?);
        }

        let tree = Components { inner: parsed };
        tree.validate()?;
        Ok(tree)
    }

    /// get all referenced media in this component tree
    pub fn get_media_refs(&self) -> Vec<MediaReference> {
        let mut refs = Vec::new();
        for c in &self.inner {
            c.ty.collect_media_refs(&mut refs);
        }
        refs
    }
}

impl<C: ComponentState> ComponentType<C> {
    fn collect_media_refs(&self, refs: &mut Vec<C::Media>) {
        match self {
            ComponentType::Container { components, .. }
            | ComponentType::Section { components, .. } => {
                for c in components {
                    c.ty().collect_media_refs(refs);
                }
            }
            ComponentType::Details {
                summary, details, ..
            } => {
                for c in summary.iter().chain(details) {
                    c.ty().collect_media_refs(refs);
                }
            }
            ComponentType::Media { items } | ComponentType::Gallery { items } => {
                for item in items {
                    refs.push(item.media.clone());
                }
            }
            _ => {}
        }
    }
}

impl<C: ComponentState> Components<C> {
    pub fn collect_media_refs(&self, refs: &mut Vec<C::Media>) {
        for c in &self.inner {
            c.ty.collect_media_refs(refs);
        }
    }
}

impl Component<Create> {
    fn parse_inner<R>(
        self,
        prev: Option<&Components<Canonical>>,
        id_allocator: &mut IdAllocator,
        resolve_media: &R,
    ) -> Result<Component<Canonical>, ApiError>
    where
        R: Fn(MediaReference) -> Result<Media, ApiError>,
    {
        // handle `Reference`s
        if let ComponentType::Reference { reference_id } = &self.ty {
            let prev_tree = prev.ok_or_else(|| {
                ApiError::with_message(
                    ErrorCode::InvalidData,
                    "Reference requires a previous version".into(),
                )
            })?;

            let referenced = prev_tree.find_by_id(*reference_id).ok_or_else(|| {
                ApiError::with_message(
                    ErrorCode::NotFound,
                    format!("Referenced component {} not found", reference_id.0),
                )
            })?;

            return if self.id == Some(referenced.id) {
                // MOVE: Return the existing canonical component
                Ok(referenced.clone())
            } else {
                // CLONE: Deep clone existing canonical component with new IDs
                referenced.clone_with_new_ids(id_allocator)
            };
        }

        let id = id_allocator.allocate(self.id);
        let ty = self.ty.try_map(
            |child| child.parse_inner(prev, id_allocator, resolve_media),
            |media| resolve_media(media),
        )?;

        Ok(Component { id, ty })
    }

    fn parse_thin_inner<R>(
        self,
        prev: Option<&Components<Thin>>,
        id_allocator: &mut IdAllocator,
        resolve_media: &R,
    ) -> Result<Component<Thin>, ApiError>
    where
        R: Fn(MediaReference) -> Result<MediaId, ApiError>,
    {
        // handle `Reference`s
        if let ComponentType::Reference { reference_id } = &self.ty {
            let prev_tree = prev.ok_or_else(|| {
                ApiError::with_message(
                    ErrorCode::InvalidData,
                    "Reference requires a previous version".into(),
                )
            })?;

            let referenced = prev_tree.find_by_id(*reference_id).ok_or_else(|| {
                ApiError::with_message(
                    ErrorCode::NotFound,
                    format!("Referenced component {} not found", reference_id.0),
                )
            })?;

            return if self.id == Some(referenced.id) {
                // MOVE: Return the existing canonical component
                Ok(referenced.clone())
            } else {
                // CLONE: Deep clone existing canonical component with new IDs
                referenced.clone_with_new_ids(id_allocator)
            };
        }

        let id = id_allocator.allocate(self.id);
        let ty = self.ty.try_map(
            |child| child.parse_thin_inner(prev, id_allocator, resolve_media),
            |media| resolve_media(media),
        )?;

        Ok(Component { id, ty })
    }
}

impl Component<Canonical> {
    fn clone_with_new_ids(
        &self,
        id_allocator: &mut IdAllocator,
    ) -> Result<Component<Canonical>, ApiError> {
        let id = id_allocator.allocate(None);

        let ty = match &self.ty {
            ComponentType::Container { components, color } => ComponentType::Container {
                components: components
                    .iter()
                    .map(|child| child.clone_with_new_ids(id_allocator))
                    .collect::<Result<Vec<_>, _>>()?,
                color: color.clone(),
            },
            ComponentType::Section { components, color } => ComponentType::Section {
                components: components
                    .iter()
                    .map(|child| child.clone_with_new_ids(id_allocator))
                    .collect::<Result<Vec<_>, _>>()?,
                color: color.clone(),
            },
            ComponentType::Details {
                open,
                color,
                summary,
                details,
            } => ComponentType::Details {
                open: *open,
                color: color.clone(),
                summary: summary
                    .iter()
                    .map(|child| child.clone_with_new_ids(id_allocator))
                    .collect::<Result<Vec<_>, _>>()?,
                details: details
                    .iter()
                    .map(|child| child.clone_with_new_ids(id_allocator))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            ComponentType::Text { content } => ComponentType::Text {
                content: content.clone(),
            },
            ComponentType::Button {
                label,
                style,
                custom_id,
            } => ComponentType::Button {
                label: label.clone(),
                style: *style,
                custom_id: custom_id.clone(),
            },
            ComponentType::LinkButton { label, url } => ComponentType::LinkButton {
                label: label.clone(),
                url: url.clone(),
            },
            ComponentType::Media { items } => ComponentType::Media {
                items: items.clone(),
            },
            ComponentType::Gallery { items } => ComponentType::Gallery {
                items: items.clone(),
            },
            ComponentType::Reference { .. } => {
                return Err(ApiError::with_message(
                    ErrorCode::InvalidData,
                    "Cannot clone a reference".into(),
                ))
            }
        };

        Ok(Component { id, ty })
    }
}

impl Component<Thin> {
    fn clone_with_new_ids(
        &self,
        id_allocator: &mut IdAllocator,
    ) -> Result<Component<Thin>, ApiError> {
        let id = id_allocator.allocate(None);

        let ty = match &self.ty {
            ComponentType::Container { components, color } => ComponentType::Container {
                components: components
                    .iter()
                    .map(|child| child.clone_with_new_ids(id_allocator))
                    .collect::<Result<Vec<_>, _>>()?,
                color: color.clone(),
            },
            ComponentType::Section { components, color } => ComponentType::Section {
                components: components
                    .iter()
                    .map(|child| child.clone_with_new_ids(id_allocator))
                    .collect::<Result<Vec<_>, _>>()?,
                color: color.clone(),
            },
            ComponentType::Details {
                open,
                color,
                summary,
                details,
            } => ComponentType::Details {
                open: *open,
                color: color.clone(),
                summary: summary
                    .iter()
                    .map(|child| child.clone_with_new_ids(id_allocator))
                    .collect::<Result<Vec<_>, _>>()?,
                details: details
                    .iter()
                    .map(|child| child.clone_with_new_ids(id_allocator))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            ComponentType::Text { content } => ComponentType::Text {
                content: content.clone(),
            },
            ComponentType::Button {
                label,
                style,
                custom_id,
            } => ComponentType::Button {
                label: label.clone(),
                style: *style,
                custom_id: custom_id.clone(),
            },
            ComponentType::LinkButton { label, url } => ComponentType::LinkButton {
                label: label.clone(),
                url: url.clone(),
            },
            ComponentType::Media { items } => ComponentType::Media {
                items: items.clone(),
            },
            ComponentType::Gallery { items } => ComponentType::Gallery {
                items: items.clone(),
            },
            ComponentType::Reference { .. } => {
                return Err(ApiError::with_message(
                    ErrorCode::InvalidData,
                    "Cannot clone a reference".into(),
                ))
            }
        };

        Ok(Component { id, ty })
    }
}

impl Components<Canonical> {
    /// look up a component by its id
    pub fn find_by_id(&self, id: ComponentId) -> Option<&Component<Canonical>> {
        self.inner.iter().find_map(|c| c.find_by_id(id))
    }
}

impl Components<Thin> {
    /// look up a component by its id
    pub fn find_by_id(&self, id: ComponentId) -> Option<&Component<Thin>> {
        self.inner.iter().find_map(|c| c.find_by_id(id))
    }
}

impl<C1: ComponentState> ComponentType<C1> {
    /// recursively transforms the state of the component tree from `C1` to `C2`.
    pub fn try_map<C2, F1, F2, E>(
        self,
        mut map_child: F1,
        mut map_media: F2,
    ) -> Result<ComponentType<C2>, E>
    where
        C2: ComponentState,
        F1: FnMut(Component<C1>) -> Result<Component<C2>, E>,
        F2: FnMut(C1::Media) -> Result<C2::Media, E>,
    {
        Ok(match self {
            ComponentType::Container { components, color } => ComponentType::Container {
                components: components
                    .into_iter()
                    .map(&mut map_child)
                    .collect::<Result<_, _>>()?,
                color,
            },
            ComponentType::Section { components, color } => ComponentType::Section {
                components: components
                    .into_iter()
                    .map(&mut map_child)
                    .collect::<Result<_, _>>()?,
                color,
            },
            ComponentType::Details {
                open,
                color,
                summary,
                details,
            } => ComponentType::Details {
                open,
                color,
                summary: summary
                    .into_iter()
                    .map(&mut map_child)
                    .collect::<Result<_, _>>()?,
                details: details
                    .into_iter()
                    .map(&mut map_child)
                    .collect::<Result<_, _>>()?,
            },

            // Map variants with Media items
            ComponentType::Media { items } => ComponentType::Media {
                items: items
                    .into_iter()
                    .map(|c| {
                        map_media(c.media).map(|media| ComponentMedia {
                            media,
                            description: c.description,
                            spoiler: c.spoiler,
                        })
                    })
                    .collect::<Result<_, _>>()?,
            },
            ComponentType::Gallery { items } => ComponentType::Gallery {
                items: items
                    .into_iter()
                    .map(|c| {
                        map_media(c.media).map(|media| ComponentMedia {
                            media,
                            description: c.description,
                            spoiler: c.spoiler,
                        })
                    })
                    .collect::<Result<_, _>>()?,
            },

            // Variants with no generic types can just be unpacked and repacked
            ComponentType::Text { content } => ComponentType::Text { content },
            ComponentType::Button {
                label,
                style,
                custom_id,
            } => ComponentType::Button {
                label,
                style,
                custom_id,
            },
            ComponentType::LinkButton { label, url } => ComponentType::LinkButton { label, url },
            ComponentType::Reference { reference_id } => ComponentType::Reference { reference_id },
        })
    }
}

impl ComponentType<Canonical> {
    pub fn into_thin(self) -> ComponentType<Thin> {
        match self {
            ComponentType::Button {
                label,
                style,
                custom_id,
            } => ComponentType::Button {
                label,
                style,
                custom_id,
            },
            ComponentType::LinkButton { label, url } => ComponentType::LinkButton { label, url },
            ComponentType::Text { content } => ComponentType::Text { content },
            ComponentType::Reference { reference_id } => ComponentType::Reference { reference_id },
            ComponentType::Container { components, color } => ComponentType::Container {
                components: components.into_iter().map(|c| c.into_thin()).collect(),
                color,
            },
            ComponentType::Section { components, color } => ComponentType::Section {
                components: components.into_iter().map(|c| c.into_thin()).collect(),
                color,
            },
            ComponentType::Details {
                open,
                color,
                summary,
                details,
            } => ComponentType::Details {
                open,
                color,
                summary: summary.into_iter().map(|c| c.into_thin()).collect(),
                details: details.into_iter().map(|c| c.into_thin()).collect(),
            },
            ComponentType::Media { items } => ComponentType::Media {
                items: items.into_iter().map(|i| i.into_thin()).collect(),
            },
            ComponentType::Gallery { items } => ComponentType::Gallery {
                items: items.into_iter().map(|i| i.into_thin()).collect(),
            },
        }
    }
}

impl Component<Thin> {
    pub fn into_canonical<F, E>(self, resolve_media: &F) -> Result<Component<Canonical>, E>
    where
        F: Fn(MediaId) -> Result<Media, E>,
    {
        let new_ty = match self.ty {
            // leaf nodes with no media
            ComponentType::Button {
                label,
                style,
                custom_id,
            } => ComponentType::Button {
                label,
                style,
                custom_id,
            },
            ComponentType::LinkButton { label, url } => ComponentType::LinkButton { label, url },
            ComponentType::Text { content } => ComponentType::Text { content },
            ComponentType::Reference { reference_id } => ComponentType::Reference { reference_id },

            // recursive containers
            ComponentType::Container { components, color } => ComponentType::Container {
                components: components
                    .into_iter()
                    .map(|c| c.into_canonical(resolve_media))
                    .collect::<Result<_, _>>()?,
                color,
            },
            ComponentType::Section { components, color } => ComponentType::Section {
                components: components
                    .into_iter()
                    .map(|c| c.into_canonical(resolve_media))
                    .collect::<Result<_, _>>()?,
                color,
            },
            ComponentType::Details {
                open,
                color,
                summary,
                details,
            } => ComponentType::Details {
                open,
                color,
                summary: summary
                    .into_iter()
                    .map(|c| c.into_canonical(resolve_media))
                    .collect::<Result<_, _>>()?,
                details: details
                    .into_iter()
                    .map(|c| c.into_canonical(resolve_media))
                    .collect::<Result<_, _>>()?,
            },

            // media nodes
            ComponentType::Media { items } => ComponentType::Media {
                items: items
                    .into_iter()
                    .map(|i| i.inflate(resolve_media))
                    .collect::<Result<_, _>>()?,
            },
            ComponentType::Gallery { items } => ComponentType::Gallery {
                items: items
                    .into_iter()
                    .map(|i| i.inflate(resolve_media))
                    .collect::<Result<_, _>>()?,
            },
        };

        Ok(Component {
            id: self.id,
            ty: new_ty,
        })
    }
}

impl ComponentMedia<MediaId> {
    fn inflate<F, E>(self, resolve_media: &F) -> Result<ComponentMedia<Media>, E>
    where
        F: Fn(MediaId) -> Result<Media, E>,
    {
        Ok(ComponentMedia {
            media: resolve_media(self.media)?,
            description: self.description,
            spoiler: self.spoiler,
        })
    }
}

impl Components<Thin> {
    pub fn into_canonical<F, E>(self, resolve_media: F) -> Result<Components<Canonical>, E>
    where
        F: Fn(MediaId) -> Result<Media, E>,
    {
        let inner = self
            .inner
            .into_iter()
            .map(|c| c.into_canonical(&resolve_media))
            .collect::<Result<Vec<_>, E>>()?;

        Ok(Components { inner })
    }

    pub fn apply_delta(
        &mut self,
        delta: FlumeDelta,
        resolve_media: impl Fn(MediaReference) -> Result<MediaId, ApiError>,
    ) -> Result<(), ApiError> {
        let mut id_allocator = IdAllocator::new();
        for c in &self.inner {
            c.visit_ids(&mut |id| id_allocator.mark_used(id.0));
        }

        // 0. process init (replace entire tree)
        if let Some(init_components) = delta.init {
            let mut init_parsed = Vec::with_capacity(init_components.inner.len());
            for c in init_components.inner {
                init_parsed.push(c.parse_thin_inner(None, &mut id_allocator, &resolve_media)?);
            }
            self.inner = init_parsed;
        }

        // 1. process deletes
        for id in delta.delete {
            self.delete_by_id(id);
        }

        // 2. process replacements
        for r in delta.replace {
            let target_id = r.target;
            let mut parsed_replacements = Vec::with_capacity(r.components.len());

            for comp_create in r.components {
                let thin =
                    comp_create.parse_thin_inner(Some(self), &mut id_allocator, &resolve_media)?;
                parsed_replacements.push(thin);
            }

            if !self.replace_by_id(target_id, parsed_replacements) {
                return Err(ApiError::with_message(
                    ErrorCode::NotFound,
                    format!("component {} not found for replacement", target_id.0),
                ));
            }
        }

        // 3. process appends
        for a in delta.append {
            let parent_id = a.target;

            let Some(parent) = self.get_mut_by_id(parent_id) else {
                return Err(ApiError::with_message(
                    ErrorCode::NotFound,
                    format!("parent component {} not found for append", parent_id.0),
                ));
            };

            for c in a.components {
                parent.append(c, &mut id_allocator, &resolve_media)?;
            }
        }

        self.validate()?;

        Ok(())
    }

    /// delete a component by its id
    fn delete_by_id(&mut self, target_id: ComponentId) -> bool {
        Self::recursive_delete(&mut self.inner, target_id)
    }

    /// helper for delete_by_id
    fn recursive_delete(components: &mut Vec<Component<Thin>>, target_id: ComponentId) -> bool {
        if let Some(pos) = components.iter().position(|c| c.id == target_id) {
            components.remove(pos);
            return true;
        }

        for c in components.iter_mut() {
            match &mut c.ty {
                ComponentType::Container {
                    components: children,
                    ..
                }
                | ComponentType::Section {
                    components: children,
                    ..
                } => {
                    if Self::recursive_delete(children, target_id) {
                        return true;
                    }
                }
                ComponentType::Details {
                    summary, details, ..
                } => {
                    if Self::recursive_delete(summary, target_id) {
                        return true;
                    }
                    if Self::recursive_delete(details, target_id) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// replace component with taret id with a sequence of new ones
    fn replace_by_id(
        &mut self,
        target_id: ComponentId,
        replacements: Vec<Component<Thin>>,
    ) -> bool {
        Self::recursive_replace(&mut self.inner, target_id, replacements)
    }

    /// recursively replace/splice a list of components
    fn recursive_replace(
        components: &mut Vec<Component<Thin>>,
        target_id: ComponentId,
        replacements: Vec<Component<Thin>>,
    ) -> bool {
        if let Some(pos) = components.iter().position(|c| c.id == target_id) {
            components.splice(pos..pos + 1, replacements);
            return true;
        }

        for c in components.iter_mut() {
            let found = match &mut c.ty {
                ComponentType::Container {
                    components: children,
                    ..
                }
                | ComponentType::Section {
                    components: children,
                    ..
                } => {
                    return Self::recursive_replace(children, target_id, replacements.clone());
                }
                ComponentType::Details {
                    summary, details, ..
                } => {
                    if Self::recursive_replace(summary, target_id, replacements.clone()) {
                        return true;
                    }
                    if Self::recursive_replace(details, target_id, replacements.clone()) {
                        return true;
                    }
                    false
                }
                _ => false,
            };

            if found {
                return true;
            }
        }

        false
    }

    /// get a mutable reference to a component from its id
    fn get_mut_by_id(&mut self, target_id: ComponentId) -> Option<&mut Component<Thin>> {
        for c in self.inner.iter_mut() {
            if let Some(res) = c.get_mut_by_id(target_id) {
                return Some(res);
            }
        }
        None
    }
}

impl Component<Thin> {
    /// get a mutable reference to a component from its id
    fn get_mut_by_id(&mut self, target_id: ComponentId) -> Option<&mut Component<Thin>> {
        if self.id == target_id {
            return Some(self);
        }

        match &mut self.ty {
            ComponentType::Container { components, .. }
            | ComponentType::Section { components, .. } => {
                for c in components.iter_mut() {
                    if let Some(res) = c.get_mut_by_id(target_id) {
                        return Some(res);
                    }
                }
            }
            ComponentType::Details {
                summary, details, ..
            } => {
                for c in summary.iter_mut() {
                    if let Some(res) = c.get_mut_by_id(target_id) {
                        return Some(res);
                    }
                }
                for c in details.iter_mut() {
                    if let Some(res) = c.get_mut_by_id(target_id) {
                        return Some(res);
                    }
                }
            }
            _ => {}
        }

        None
    }
}
