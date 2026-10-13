//! logic for validating components

use std::collections::HashSet;

use crate::v2::types::components::{
    Component, ComponentCustomId, ComponentId, ComponentType, Components, Create,
    interactive::{Label, Validation},
};

// TODO: more validation
// - validate text length with both byte length and graphemes
// - validate that components don't have duplicate ids (ie. two components cant have the same id)
// - validate that components aren't referenced more than once (ie. the same component cant be in two different containers)
// - validate that cycles dont exist (ie. components must form an acyclic tree)
// - validate ComponentType::{is_usable_inline, requires_form}
// - validate that forms, rows cannot be nested

// TODO: derive default
pub struct ValidationState {
    path: Vec<String>,
    errors: Vec<ParseField>,
    limits: Limits,
    known_ids: HashSet<ComponentId>,
    used_ids: HashSet<ComponentId>,
    seen_ids: HashSet<ComponentId>,
    seen_custom_ids: HashSet<ComponentCustomId>,
}

// struct ValidationPath {
//     path: String,
//     context: ValidationContext,
// }

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

/// restrictions on a component tree
#[derive(Debug, Clone)]
pub struct Limits {
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

pub struct ParseOutput {
    pub invalid: Components,
    pub errors: Vec<ParseField>,
}

pub struct ParseField {
    /// path to this field inside the component object
    pub key: Vec<String>,

    /// the errors that occured
    pub errors: Vec<ParseError>,
}

/// an error that occured while parsing/validating components
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("at least one root component is required")]
    MissingRoot,

    #[error("component id {0:?} already used")]
    DuplicateId(ComponentId),

    #[error("component custom id {0:?} already used")]
    DuplicateCustomId(ComponentCustomId),

    #[error("component {0:?} doesnt exist")]
    UnknownComponent(ComponentId),

    #[error("label text must be between {min} and {max} characters")]
    LabelTextLength { min: u64, max: u64 },

    #[error("label description must be between {min} and {max} characters")]
    LabelDescriptionLength { min: u64, max: u64 },

    #[error("custom_id must be between {min} and {max} characters")]
    CustomIdLength { min: u64, max: u64 },

    #[error("text content too long (max {max})")]
    TextLength { max: u64 },

    #[error("row can have max {max} components")]
    RowChildrenLength { max: u64 },

    #[error("items must be between {min} and {max}")]
    GalleryItemsLength { min: u64, max: u64 },
}

// TODO: maybe use this trait?
// trait Visitor {
//     fn enter<F>(&mut self, segment: impl Into<String>, f: F)
//     where
//         F: FnOnce(&mut Self);
//
//     fn limits(&self) -> &Limits;
//
//     // visit label
//     // visit custom id
//     // visit etc
// }

impl ValidationState {
    fn new(limits: Limits) -> Self {
        Self {
            path: vec![],
            errors: vec![],
            limits,
            seen_ids: HashSet::new(),
            seen_custom_ids: HashSet::new(),
            known_ids: HashSet::new(),
            used_ids: HashSet::new(),
        }
    }

    fn enter<F>(&mut self, segment: impl Into<String>, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.path.push(segment.into());
        f(self);
        self.path.pop();
    }

    fn enter_index<F>(&mut self, index: usize, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.enter(index.to_string(), f)
    }

    fn push_error(&mut self, error: ParseError) {
        if let Some(field) = self.errors.iter_mut().find(|f| f.key == self.path) {
            field.errors.push(error);
        } else {
            self.errors.push(ParseField {
                key: self.path.clone(),
                errors: vec![error],
            });
        }
    }

    fn has_component(&self, id: &ComponentId) -> bool {
        self.known_ids.contains(id)
    }

    fn validate_label(&mut self, label: &Label) {
        let max_label = self.limits.text_label as u64;
        if label.text.is_empty() || label.text.len() as u64 > max_label {
            self.push_error(ParseError::LabelTextLength {
                min: 1,
                max: max_label,
            });
        }
        if let Some(desc) = &label.description {
            let max_desc = self.limits.text_description as u64;
            if desc.is_empty() || desc.len() as u64 > max_desc {
                self.push_error(ParseError::LabelDescriptionLength {
                    min: 1,
                    max: max_desc,
                });
            }
        }
    }

    // TODO: impl validate for ComponentCustomId
    fn validate_custom_id(&mut self, custom_id: &ComponentCustomId) {
        if custom_id.0.is_empty() || custom_id.0.len() > 128 {
            self.push_error(ParseError::CustomIdLength { min: 1, max: 128 });
        }
    }

    fn validate_validation(&mut self, _component_type: &ComponentType, _validation: &Validation) {
        // TODO: Implement more robust validation for `Validation` struct
        todo!()
    }
}

impl Components {
    fn parse(self, mut state: ValidationState) -> Result<Self, ParseOutput> {
        // TODO: populate state.known_ids

        if self.roots.is_empty() {
            state.push_error(ParseError::MissingRoot);
        }

        for (i, root_id) in self.roots.iter().enumerate() {
            state.enter_index(i, |s| {
                if !s.has_component(root_id) {
                    s.push_error(ParseError::UnknownComponent(*root_id));
                }
            });
        }

        for (i, component) in self.items.iter().enumerate() {
            state.enter_index(i, |s| component.validate_inner(s));
        }

        if state.errors.is_empty() {
            Ok(self)
        } else {
            Err(ParseOutput {
                invalid: self,
                errors: state.errors,
            })
        }
    }
}

// TODO: impl ParseErrors to ApiError conersion

impl Component {
    fn validate_inner(&self, state: &mut ValidationState) {
        // TODO: check for duplicate ids
        // state.seen_ids.insert(self.id);
        // state.push_error(ParseError::DuplicateComponentId(component.id));

        // TODO: Implement recursive validation based on component type
        self.ty.validate_inner(state);
    }
}

impl ComponentType {
    fn validate_inner(&self, state: &mut ValidationState) {
        match self {
            ComponentType::Button(button) => {
                state.enter("label", |s| s.validate_label(&button.label));
                // TODO: Validate action
            }
            ComponentType::Input(input) => {
                state.enter("custom_id", |s| s.validate_custom_id(&input.custom_id));
                state.enter("label", |s| s.validate_label(&input.label));
                state.enter("validation", |s| {
                    s.validate_validation(self, &input.validation)
                });
            }
            ComponentType::Textarea(textarea) => {
                state.enter("custom_id", |s| s.validate_custom_id(&textarea.custom_id));
                state.enter("label", |s| s.validate_label(&textarea.label));
                state.enter("validation", |s| {
                    s.validate_validation(self, &textarea.validation)
                });
            }
            ComponentType::Select(select) => {
                state.enter("custom_id", |s| s.validate_custom_id(&select.custom_id));
                state.enter("label", |s| s.validate_label(&select.label));
                state.enter("validation", |s| {
                    s.validate_validation(self, &select.validation)
                });
            }
            ComponentType::Upload(upload) => {
                state.enter("custom_id", |s| s.validate_custom_id(&upload.custom_id));
                state.enter("label", |s| s.validate_label(&upload.label));
                state.enter("validation", |s| {
                    s.validate_validation(self, &upload.validation)
                });
            }
            ComponentType::Checkbox(checkbox) => {
                state.enter("custom_id", |s| s.validate_custom_id(&checkbox.custom_id));
                state.enter("option", |s| s.validate_label(&checkbox.choice.label));
                state.enter("validation", |s| {
                    s.validate_validation(self, &checkbox.validation)
                });
            }
            ComponentType::Checkboxes(checkboxes) => {
                state.enter("custom_id", |s| s.validate_custom_id(&checkboxes.custom_id));
                state.enter("label", |s| s.validate_label(&checkboxes.label));
                state.enter("validation", |s| {
                    s.validate_validation(self, &checkboxes.validation)
                });
            }
            ComponentType::Container(container) => {
                self.validate_child_ids(&container.components, "components", state);
            }
            ComponentType::Text(text) => {
                let max_text = state.limits.text_component as u64;
                if text.content.len() as u64 > max_text {
                    state.push_error(ParseError::TextLength { max: max_text });
                }
            }
            ComponentType::Details(details) => {
                self.validate_child_ids(&details.children, "children", state);
            }
            ComponentType::Section(section) => {
                self.validate_child_ids(&section.components, "components", state);
            }
            ComponentType::Form(form) => {
                state.enter("custom_id", |s| s.validate_custom_id(&form.custom_id));
                self.validate_child_ids(&form.components, "components", state);
                // TODO: Validate no nested forms
            }
            ComponentType::Row(row) => {
                if row.components.len() > 5 {
                    state.push_error(ParseError::RowChildrenLength { max: 5 });
                }
                self.validate_child_ids(&row.components, "components", state);
            }
            ComponentType::Media(_) => {
                // nothing to validate
            }
            ComponentType::Gallery(gallery) => {
                if gallery.items.is_empty() || gallery.items.len() > 20 {
                    state.push_error(ParseError::GalleryItemsLength { min: 1, max: 20 });
                }
            }
            ComponentType::Reference(reference) => {
                if !state.has_component(&reference.reference_id) {
                    state.push_error(ParseError::UnknownComponent(reference.reference_id));
                }
            }
            ComponentType::Template(_) => {
                // TODO: Implement
            }
        }
    }

    fn validate_child_ids(&self, ids: &[ComponentId], segment: &str, state: &mut ValidationState) {
        state.enter(segment, |s| {
            for (i, id) in ids.iter().enumerate() {
                s.enter_index(i, |s2| {
                    if !s2.has_component(id) {
                        s2.push_error(ParseError::UnknownComponent(*id));
                    }
                });
            }
        });
    }
}

impl Create {
    /// parse and validate
    pub fn parse(self, limits: Limits) -> Result<Components, ParseOutput> {
        self.inner.parse(ValidationState::new(limits))
    }
}

impl Limits {
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
