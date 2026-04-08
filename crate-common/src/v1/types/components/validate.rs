use super::types::*;
use crate::v1::types::error::{ApiError, ErrorCode, ErrorField, ErrorFieldType};

/// Tracks validation state across the component tree.
struct ValidationState {
    path: Vec<String>,
    component_count: usize,
    depth: usize,
    total_text_length: usize,
}

impl ValidationState {
    /// Create a new validation state.
    pub fn new() -> ValidationState {
        ValidationState {
            path: vec![],
            component_count: 0,
            depth: 1,
            total_text_length: 0,
        }
    }

    /// Get the current path in the tree.
    pub fn current_path(&self) -> Vec<String> {
        self.path.clone()
    }

    /// Executes a closure at a deeper level and automatically pops the path.
    pub fn enter<F>(&mut self, segment: String, f: F) -> Vec<ErrorField>
    where
        F: FnOnce(&mut Self) -> Vec<ErrorField>,
    {
        self.path.push(segment);
        let res = f(self);
        self.path.pop();
        res
    }

    /// Executes a closure for a specific child index.
    pub fn enter_index<F>(&mut self, index: usize, f: F) -> Vec<ErrorField>
    where
        F: FnOnce(&mut Self) -> Vec<ErrorField>,
    {
        self.enter(index.to_string(), f)
    }

    /// Enter a component and check for count and depth limits.
    pub fn enter_component(&mut self) -> Vec<ErrorField> {
        self.component_count += 1;
        let mut errs = vec![];
        if self.component_count > MAX_COMPONENTS {
            errs.push(ErrorField {
                key: self.current_path(),
                message: format!("component tree exceeds maximum of {MAX_COMPONENTS} components"),
                ty: ErrorFieldType::length(1, MAX_COMPONENTS as u64),
            });
        }
        if self.depth > MAX_DEPTH {
            errs.push(ErrorField {
                key: self.current_path(),
                message: format!("component tree exceeds maximum depth of {MAX_DEPTH}"),
                ty: ErrorFieldType::length(1, MAX_DEPTH as u64),
            });
        }
        errs
    }

    /// Add text length to the total and check for limits.
    pub fn add_text(&mut self, len: usize) -> Vec<ErrorField> {
        self.total_text_length += len;
        if self.total_text_length > MAX_TOTAL_TEXT_LENGTH {
            vec![ErrorField {
                key: self.current_path(),
                message: format!(
                    "total text length exceeds maximum of {MAX_TOTAL_TEXT_LENGTH} bytes"
                ),
                ty: ErrorFieldType::length(1, MAX_TOTAL_TEXT_LENGTH as u64),
            }]
        } else {
            vec![]
        }
    }
}

impl<C: IsComponent<C, M>, M> ComponentType<C, M> {
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

    /// Validate the component type recursively.
    pub fn validate(&self) -> Result<(), ApiError> {
        let mut state = ValidationState::new();
        let errs = self.validate_inner(&mut state);

        if errs.is_empty() {
            Ok(())
        } else {
            Err(ApiError {
                fields: errs,
                ..ApiError::with_message(
                    ErrorCode::InvalidData,
                    "invalid component data".to_owned(),
                )
            })
        }
    }

    /// Recursively validate the component type and its children.
    fn validate_inner(&self, state: &mut ValidationState) -> Vec<ErrorField> {
        let mut errs = state.enter_component();

        match self {
            ComponentType::Button {
                label, custom_id, ..
            } => {
                errs.extend(validate_length(
                    &custom_id.0,
                    1,
                    128,
                    "custom_id",
                    state.current_path(),
                ));
                errs.extend(state.add_text(custom_id.0.len()));
                errs.extend(validate_length(
                    label,
                    1,
                    256,
                    "label",
                    state.current_path(),
                ));
                errs.extend(state.add_text(label.len()));
            }
            ComponentType::LinkButton { label, url, .. } => {
                errs.extend(validate_length(
                    label,
                    1,
                    256,
                    "label",
                    state.current_path(),
                ));
                errs.extend(state.add_text(label.len()));
                if let Some(u) = url {
                    errs.extend(state.add_text(u.as_str().len()));
                }
            }
            ComponentType::Container { components, .. } => {
                errs.extend(validate_count(
                    components.len(),
                    1,
                    20,
                    "container components",
                    state.current_path(),
                ));
                state.depth += 1;
                for (i, c) in components.iter().enumerate() {
                    errs.extend(state.enter_index(i, |s| {
                        let mut e = vec![];
                        if !matches!(
                            c.ty(),
                            ComponentType::Button { .. } | ComponentType::LinkButton { .. }
                        ) {
                            e.push(ErrorField {
                                key: s.current_path(),
                                message: "Containers can only contain Buttons or LinkButtons"
                                    .into(),
                                ty: ErrorFieldType::Other,
                            });
                        }
                        e.extend(c.ty().validate_inner(s));
                        e
                    }));
                }
                state.depth -= 1;
            }
            ComponentType::Section { components, .. } => {
                errs.extend(validate_count(
                    components.len(),
                    1,
                    20,
                    "section components",
                    state.current_path(),
                ));
                state.depth += 1;
                for (i, c) in components.iter().enumerate() {
                    errs.extend(state.enter_index(i, |s| c.ty().validate_inner(s)));
                }
                state.depth -= 1;
            }
            ComponentType::Text { content } => {
                errs.extend(validate_length(
                    content,
                    1,
                    8192,
                    "text",
                    state.current_path(),
                ));
                errs.extend(state.add_text(content.len()));
            }
            ComponentType::Details {
                summary, details, ..
            } => {
                errs.extend(validate_count(
                    summary.len(),
                    1,
                    20,
                    "summary components",
                    state.current_path(),
                ));
                errs.extend(state.enter("summary".into(), |s| {
                    let mut e = vec![];
                    s.depth += 1;
                    for (i, c) in summary.iter().enumerate() {
                        e.extend(s.enter_index(i, |s2| c.ty().validate_inner(s2)));
                    }
                    s.depth -= 1;
                    e
                }));

                errs.extend(validate_count(
                    details.len(),
                    1,
                    20,
                    "details components",
                    state.current_path(),
                ));
                errs.extend(state.enter("details".into(), |s| {
                    let mut e = vec![];
                    s.depth += 1;
                    for (i, c) in details.iter().enumerate() {
                        e.extend(s.enter_index(i, |s2| c.ty().validate_inner(s2)));
                    }
                    s.depth -= 1;
                    e
                }));
            }
            ComponentType::Media { items } | ComponentType::Gallery { items } => {
                let label = if matches!(self, ComponentType::Media { .. }) {
                    "media"
                } else {
                    "gallery"
                };
                errs.extend(validate_count(
                    items.len(),
                    1,
                    20,
                    &format!("{label} items"),
                    state.current_path(),
                ));
                state.depth += 1;
                for (i, item) in items.iter().enumerate() {
                    errs.extend(state.enter_index(i, |s| {
                        let mut e = vec![];
                        if let Some(desc) = &item.description {
                            e.extend(validate_length(
                                desc,
                                1,
                                1024,
                                "description",
                                s.current_path(),
                            ));
                            e.extend(s.add_text(desc.len()));
                        }
                        e
                    }));
                }
                state.depth -= 1;
            }
            ComponentType::Reference { .. } => {}
        }
        errs
    }
}

fn validate_count(
    count: usize,
    min: usize,
    max: usize,
    field: &str,
    path: Vec<String>,
) -> Vec<ErrorField> {
    if count < min {
        vec![ErrorField {
            key: path,
            message: format!("{field} cannot be empty"),
            ty: ErrorFieldType::length(min as u64, max as u64),
        }]
    } else if count > max {
        vec![ErrorField {
            key: path,
            message: format!("{field} can have up to {max}"),
            ty: ErrorFieldType::length(min as u64, max as u64),
        }]
    } else {
        vec![]
    }
}

fn validate_length(
    value: &str,
    min: usize,
    max: usize,
    field: &str,
    path: Vec<String>,
) -> Vec<ErrorField> {
    if value.is_empty() {
        vec![ErrorField {
            key: path,
            message: format!("{field} cannot be empty"),
            ty: ErrorFieldType::length(min as u64, max as u64),
        }]
    } else if value.len() > max {
        vec![ErrorField {
            key: path,
            message: format!("{field} can have up to {max} chars"),
            ty: ErrorFieldType::length(min as u64, max as u64),
        }]
    } else {
        vec![]
    }
}

impl<M> Component<M> {
    /// Append another component to this component tree.
    ///
    /// ## rules
    ///
    /// - Text can be appended to other Text (content is concatenated)
    /// - Media can be appended to Gallery (added to items)
    /// - any component can be appended to Container and Section
    /// - any component can be appended to Details. it will be appended to `details`, not `summary`.
    pub fn append(&mut self, other: Component<M>) -> Result<(), ApiError> {
        match &mut self.ty {
            ComponentType::Text { content } => {
                if let ComponentType::Text {
                    content: other_content,
                } = other.ty
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
                if let ComponentType::Media { items: other_items } = other.ty {
                    items.extend(other_items);
                } else {
                    return Err(ApiError::with_message(
                        ErrorCode::InvalidData,
                        "only Media can be appended to Gallery".to_owned(),
                    ));
                }
            }
            ComponentType::Container { components, .. } => {
                components.push(other);
            }
            ComponentType::Section { components, .. } => {
                components.push(other);
            }
            ComponentType::Details { details, .. } => {
                details.push(other);
            }
            ComponentType::Button { .. }
            | ComponentType::LinkButton { .. }
            | ComponentType::Reference { .. }
            | ComponentType::Media { .. } => {
                return Err(ApiError::with_message(
                    ErrorCode::InvalidData,
                    "cannot append to this component type".to_owned(),
                ));
            }
        }

        self.ty.validate()
    }
}
