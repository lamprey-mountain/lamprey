use crate::{
    v1::types::error::{ApiError, ApiResult, ErrorCode, ErrorField, ErrorFieldType},
    v2::types::components::{
        Component, ComponentCustomId, ComponentId, ComponentType, Components,
        components::ComponentsCreate,
        interactive::{Label, Validation},
    },
};

// TODO: more validation
// - validate text length with both byte length and graphemes
// - validate that components don't have duplicate ids (ie. two components cant have the same id)
// - validate that components aren't referenced more than once (ie. the same component cant be in two different containers)
// - validate that cycles dont exist (ie. components must form an acyclic tree)
// - validate ComponentType::{is_usable_inline, requires_form}
// - validate that forms, rows cannot be nested

pub struct ValidationState<'a> {
    path: Vec<String>,
    errors: Vec<ErrorField>,
    components: &'a Components,
    // TODO: use ComponentsLimits
    // TODO: use ValidationContext
}

// alternatively, i could have is_inside_row: bool, is_inside_form
#[derive(Debug, Clone, Copy)]
enum ValidationContext {
    /// we're not inside any specific context
    Root,

    /// we're inside a form
    Form,

    /// we're inside a row
    Row,
}

impl<'a> ValidationState<'a> {
    pub fn new(components: &'a Components) -> Self {
        Self {
            path: vec![],
            errors: vec![],
            components,
        }
    }

    pub fn enter<F>(&mut self, segment: impl Into<String>, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.path.push(segment.into());
        f(self);
        self.path.pop();
    }

    pub fn enter_index<F>(&mut self, index: usize, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.enter(index.to_string(), f)
    }

    pub fn push_error(&mut self, message: String, ty: ErrorFieldType) {
        self.errors.push(ErrorField {
            key: self.path.clone(),
            message,
            ty,
        });
    }

    pub fn has_component(&self, id: &ComponentId) -> bool {
        self.components.items.iter().any(|c| &c.id == id)
    }

    pub fn validate_label(&mut self, label: &Label) {
        if label.text.is_empty() || label.text.len() > 256 {
            self.push_error(
                "label text must be between 1 and 256 characters".to_owned(),
                ErrorFieldType::Length {
                    min: Some(1),
                    max: Some(256),
                },
            );
        }
        if let Some(desc) = &label.description {
            if desc.is_empty() || desc.len() > 2048 {
                self.push_error(
                    "label description must be between 1 and 2048 characters".to_owned(),
                    ErrorFieldType::Length {
                        min: Some(1),
                        max: Some(2048),
                    },
                );
            }
        }
    }

    // TODO: impl validate for ComponentCustomId
    pub fn validate_custom_id(&mut self, custom_id: &ComponentCustomId) {
        if custom_id.0.is_empty() || custom_id.0.len() > 128 {
            self.push_error(
                "custom_id must be between 1 and 128 characters".to_owned(),
                ErrorFieldType::Length {
                    min: Some(1),
                    max: Some(128),
                },
            );
        }
    }

    pub fn validate_validation(
        &mut self,
        _component_type: &ComponentType,
        _validation: &Validation,
    ) {
        // TODO: Implement more robust validation for `Validation` struct
        todo!()
    }
}

impl Components {
    pub fn validate(&self) -> ApiResult<()> {
        let mut state = ValidationState::new(self);

        if self.roots.is_empty() {
            state.push_error(
                "at least one root component is required".to_owned(),
                ErrorFieldType::Other,
            );
        }

        for (i, root_id) in self.roots.iter().enumerate() {
            state.enter_index(i, |s| {
                if !s.has_component(root_id) {
                    s.push_error(
                        format!("root component {} not found", root_id.0),
                        ErrorFieldType::Other,
                    );
                }
            });
        }

        for (i, component) in self.items.iter().enumerate() {
            state.enter_index(i, |s| component.validate_inner(s));
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

impl Component {
    fn validate_inner(&self, state: &mut ValidationState) {
        // TODO: Implement recursive validation based on component type
        self.ty.validate_inner(state);
    }
}

impl ComponentType {
    fn validate_inner(&self, state: &mut ValidationState) {
        match self {
            ComponentType::Button {
                label,
                style: _,
                action: _,
            } => {
                state.enter("label", |s| s.validate_label(label));
                // TODO: Validate action
            }
            ComponentType::Input {
                custom_id,
                label,
                value: _,
                placeholder: _,
                validation,
            } => {
                state.enter("custom_id", |s| s.validate_custom_id(custom_id));
                state.enter("label", |s| s.validate_label(label));
                state.enter("validation", |s| s.validate_validation(self, validation));
            }
            ComponentType::Textarea {
                custom_id,
                label,
                style: _,
                value: _,
                placeholder: _,
                validation,
            } => {
                state.enter("custom_id", |s| s.validate_custom_id(custom_id));
                state.enter("label", |s| s.validate_label(label));
                state.enter("validation", |s| s.validate_validation(self, validation));
            }
            ComponentType::Select {
                custom_id,
                label,
                placeholder: _,
                dataset: _,
                validation,
            } => {
                state.enter("custom_id", |s| s.validate_custom_id(custom_id));
                state.enter("label", |s| s.validate_label(label));
                state.enter("validation", |s| s.validate_validation(self, validation));
            }
            ComponentType::Upload {
                custom_id,
                label,
                validation,
            } => {
                state.enter("custom_id", |s| s.validate_custom_id(custom_id));
                state.enter("label", |s| s.validate_label(label));
                state.enter("validation", |s| s.validate_validation(self, validation));
            }
            ComponentType::Checkbox {
                custom_id,
                option,
                validation,
            } => {
                state.enter("custom_id", |s| s.validate_custom_id(custom_id));
                state.enter("option", |s| s.validate_label(&option.label));
                state.enter("validation", |s| s.validate_validation(self, validation));
            }
            ComponentType::Checkboxes {
                custom_id,
                label,
                options: _,
                validation,
            } => {
                state.enter("custom_id", |s| s.validate_custom_id(custom_id));
                state.enter("label", |s| s.validate_label(label));
                state.enter("validation", |s| s.validate_validation(self, validation));
            }
            ComponentType::Container {
                components,
                color: _,
            } => {
                self.validate_child_ids(components, "components", state);
            }
            ComponentType::Text { content } => {
                if content.len() > 8192 {
                    state.push_error(
                        "text content too long".to_owned(),
                        ErrorFieldType::Length {
                            min: None,
                            max: Some(8192),
                        },
                    );
                }
            }
            ComponentType::Details {
                open: _,
                color: _,
                summary,
                details,
            } => {
                self.validate_child_ids(summary, "summary", state);
                self.validate_child_ids(details, "details", state);
            }
            ComponentType::Section {
                color: _,
                components,
            } => {
                self.validate_child_ids(components, "components", state);
            }
            ComponentType::Form {
                custom_id,
                components,
            } => {
                state.enter("custom_id", |s| s.validate_custom_id(custom_id));
                self.validate_child_ids(components, "components", state);
                // TODO: Validate no nested forms
            }
            ComponentType::Row { components } => {
                if components.len() > 5 {
                    state.push_error(
                        "row can have max 5 components".to_owned(),
                        ErrorFieldType::Length {
                            min: None,
                            max: Some(5),
                        },
                    );
                }
                self.validate_child_ids(components, "components", state);
            }
            ComponentType::Media { item: _ } => {
                // nothing to validate
            }
            ComponentType::Gallery { items } => {
                if items.is_empty() || items.len() > 20 {
                    state.push_error(
                        "items must be between 1 and 20".to_owned(),
                        ErrorFieldType::Length {
                            min: Some(1),
                            max: Some(20),
                        },
                    );
                }
            }
            ComponentType::Reference { reference_id } => {
                if !state.has_component(reference_id) {
                    state.push_error(
                        format!("referenced component {} not found", reference_id.0),
                        ErrorFieldType::Other,
                    );
                }
            }
            ComponentType::Template { .. } => {
                // TODO: Implement
            }
        }
    }

    fn validate_child_ids(&self, ids: &[ComponentId], segment: &str, state: &mut ValidationState) {
        state.enter(segment, |s| {
            for (i, id) in ids.iter().enumerate() {
                s.enter_index(i, |s2| {
                    if !s2.has_component(id) {
                        s2.push_error(
                            format!("component {} not found", id.0),
                            ErrorFieldType::Other,
                        );
                    }
                });
            }
        });
    }
}

pub struct ComponentsParseError {
    pub data: ComponentsCreate,
    pub errors: Vec<ErrorField>,
}

impl ComponentsCreate {
    /// parse and validate
    pub fn parse(self) -> Result<Components, ComponentsParseError> {
        todo!()
    }
}

/// restrictions on a component tree
#[derive(Debug, Clone)]
pub struct ComponentsLimits {
    /// maximum number of components in the root
    pub components_root: usize,

    /// maximum number of components in a container component (eg. section, container)
    pub components_container: usize,

    /// maximum number of components in a component tree
    pub components_total: usize,

    /// maximum length of text for labels (eg. buttons)
    pub text_label: usize,

    /// maximum length of text for label descriptions
    pub text_description: usize,

    /// maximum length of text for a text component
    pub text_component: usize,

    /// maximum length of text in a component tree
    pub text_total: usize,

    /// whether this tree can contain interactive components
    pub allow_interactive: bool,
}

impl ComponentsLimits {
    pub fn default_inert() -> Self {
        Self {
            components_root: 16,
            components_container: 64,
            components_total: 64,
            text_label: 256,
            text_description: 2048,
            text_component: 8192,
            text_total: 8192,
            allow_interactive: false,
        }
    }

    pub fn default_interactive() -> Self {
        Self {
            allow_interactive: true,
            ..Self::default_inert()
        }
    }
}
