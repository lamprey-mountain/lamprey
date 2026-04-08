use super::types::*;
use crate::v1::types::error::{ApiError, ErrorCode};

/// validate and assign ids
///
/// if there is a previous version of a component tree, it is used to resolve
/// `Reference` components.
pub fn parse_component_create<M: Clone>(
    c: ComponentCreate<M>,
    prev: Option<&Component<M>>,
) -> Result<Component<M>, ApiError> {
    let mut id_allocator = IdAllocator::new();

    // Collect existing IDs from prev tree
    if let Some(prev) = prev {
        prev.visit_ids(&mut |id| id_allocator.mark_used(id.0));
    }

    parse_component_create_inner(c, prev, &mut id_allocator)
}

struct IdAllocator {
    next_id: u16,
    used: std::collections::HashSet<u16>,
}

impl IdAllocator {
    /// Create a new ID allocator.
    fn new() -> Self {
        Self {
            next_id: 0,
            used: std::collections::HashSet::new(),
        }
    }

    /// Mark an ID as used.
    fn mark_used(&mut self, id: u16) {
        self.used.insert(id);
    }

    /// Allocate a new ID or use the requested one.
    fn allocate(&mut self, requested: Option<ComponentId>) -> ComponentId {
        if let Some(id) = requested {
            self.mark_used(id.0);
            return id;
        }
        while self.used.contains(&self.next_id) {
            if self.next_id == u16::MAX {
                // Should never happen given MAX_COMPONENTS = 64,
                // but needed for memory safety.
                panic!("Component ID namespace exhausted");
            }
            self.next_id += 1;
        }
        let id = ComponentId(self.next_id);
        self.mark_used(self.next_id);
        self.next_id += 1;
        id
    }
}

/// Recursively parse a ComponentCreate into a Component.
fn parse_component_create_inner<M: Clone>(
    c: ComponentCreate<M>,
    prev: Option<&Component<M>>,
    id_allocator: &mut IdAllocator,
) -> Result<Component<M>, ApiError> {
    if let ComponentType::Reference { reference_id } = c.ty {
        let prev_tree = prev.ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::InvalidData,
                "Reference components require a previous version".to_owned(),
            )
        })?;

        let referenced = prev_tree.find_by_id(reference_id).ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::NotFound,
                format!("Referenced component with id {} not found", reference_id.0),
            )
        })?;

        return if c.id == Some(referenced.id) {
            // Move: keep the same component
            Ok(referenced.clone())
        } else {
            // Clone: deep clone with new ids
            clone_component(referenced, id_allocator)
        };
    }

    let id = id_allocator.allocate(c.id);

    let ty = match c.ty {
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
        ComponentType::Container { components, color } => {
            let mut parsed_components = Vec::with_capacity(components.len());
            for child in components {
                parsed_components.push(parse_component_create_inner(child, prev, id_allocator)?);
            }
            ComponentType::Container {
                components: parsed_components,
                color,
            }
        }
        ComponentType::Section { components, color } => {
            let mut parsed_components = Vec::with_capacity(components.len());
            for child in components {
                parsed_components.push(parse_component_create_inner(child, prev, id_allocator)?);
            }
            ComponentType::Section {
                components: parsed_components,
                color,
            }
        }
        ComponentType::Details {
            open,
            color,
            summary,
            details,
        } => {
            let mut parsed_summary = Vec::with_capacity(summary.len());
            for child in summary {
                parsed_summary.push(parse_component_create_inner(child, prev, id_allocator)?);
            }
            let mut parsed_details = Vec::with_capacity(details.len());
            for child in details {
                parsed_details.push(parse_component_create_inner(child, prev, id_allocator)?);
            }
            ComponentType::Details {
                open,
                color,
                summary: parsed_summary,
                details: parsed_details,
            }
        }
        ComponentType::Media { items } => ComponentType::Media { items },
        ComponentType::Gallery { items } => ComponentType::Gallery { items },
        ComponentType::Reference { .. } => unreachable!("handled above"),
    };

    Ok(Component { id, ty })
}

/// Recursively clone a component and assign new IDs.
fn clone_component<M: Clone>(
    component: &Component<M>,
    id_allocator: &mut IdAllocator,
) -> Result<Component<M>, ApiError> {
    let new_id = id_allocator.allocate(None);

    let ty = match &component.ty {
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
        ComponentType::Text { content } => ComponentType::Text {
            content: content.clone(),
        },
        ComponentType::Container { components, color } => {
            let mut cloned = Vec::with_capacity(components.len());
            for child in components {
                cloned.push(clone_component(child, id_allocator)?);
            }
            ComponentType::Container {
                components: cloned,
                color: color.clone(),
            }
        }
        ComponentType::Section { components, color } => {
            let mut cloned = Vec::with_capacity(components.len());
            for child in components {
                cloned.push(clone_component(child, id_allocator)?);
            }
            ComponentType::Section {
                components: cloned,
                color: color.clone(),
            }
        }
        ComponentType::Details {
            open,
            color,
            summary,
            details,
        } => {
            let mut cloned_summary = Vec::with_capacity(summary.len());
            for child in summary {
                cloned_summary.push(clone_component(child, id_allocator)?);
            }
            let mut cloned_details = Vec::with_capacity(details.len());
            for child in details {
                cloned_details.push(clone_component(child, id_allocator)?);
            }
            ComponentType::Details {
                open: *open,
                color: color.clone(),
                summary: cloned_summary,
                details: cloned_details,
            }
        }
        ComponentType::Media { items } => ComponentType::Media {
            items: items.clone(),
        },
        ComponentType::Gallery { items } => ComponentType::Gallery {
            items: items.clone(),
        },
        ComponentType::Reference { .. } => {
            return Err(ApiError::with_message(
                ErrorCode::InvalidData,
                "Cannot clone a Reference component".to_owned(),
            ));
        }
    };

    Ok(Component { id: new_id, ty })
}
