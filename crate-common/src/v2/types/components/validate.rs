use crate::{
    v1::types::error::{ApiError, ApiResult, ErrorCode, ErrorField, ErrorFieldType},
    v2::types::components::ComponentCustomId,
    v2::types::components::interactive::{Label, Validation},
    v2::types::components::{Component, ComponentId, ComponentType, Components},
};

pub struct ValidationState<'a> {
    path: Vec<String>,
    // TODO: make this a getter/method
    pub errors: Vec<ErrorField>,
    components: &'a Components,
    // TODO: limit total number of components
    // TODO: limit total text length
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

    pub fn validate_validation(&mut self, _validation: &Validation) {
        // TODO: Implement more robust validation for `Validation` struct
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
                state.enter("validation", |s| s.validate_validation(validation));
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
                state.enter("validation", |s| s.validate_validation(validation));
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
                state.enter("validation", |s| s.validate_validation(validation));
            }
            ComponentType::Upload {
                custom_id,
                label,
                validation,
            } => {
                state.enter("custom_id", |s| s.validate_custom_id(custom_id));
                state.enter("label", |s| s.validate_label(label));
                state.enter("validation", |s| s.validate_validation(validation));
            }
            ComponentType::Checkbox {
                custom_id,
                option,
                validation,
            } => {
                state.enter("custom_id", |s| s.validate_custom_id(custom_id));
                state.enter("option", |s| s.validate_label(&option.label));
                state.enter("validation", |s| s.validate_validation(validation));
            }
            ComponentType::Checkboxes {
                custom_id,
                label,
                options: _,
                validation,
            } => {
                state.enter("custom_id", |s| s.validate_custom_id(custom_id));
                state.enter("label", |s| s.validate_label(label));
                state.enter("validation", |s| s.validate_validation(validation));
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
            ComponentType::Media { items } | ComponentType::Gallery { items } => {
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
