//! logic for validating components

use std::collections::{HashMap, HashSet};

#[cfg(feature = "regex_validation")]
use regex::Regex;

use crate::{
    v1::types::metadata::Metadata,
    v2::types::{
        MediaId,
        components::{
            Component, ComponentCustomId, ComponentId, ComponentMedia, ComponentType, Components,
            Create,
            interactive::{Allow, ButtonAction, Choice, Dataset, InputFormat, Label, Validation},
            types::Button,
        },
        media::{Media as ApiMedia, MediaReference},
    },
};

/// maximum number of keys in [`Metadata`]
const METADATA_MAX_KEYS: usize = 8;
/// maximum length of a [`Metadata`] key
const METADATA_MAX_KEY_LEN: usize = 32;
/// maximum length of a [`Metadata`] value
const METADATA_MAX_VALUE_LEN: usize = 1024;
/// maximum length of all [`Metadata`] values combined
const METADATA_MAX_VALUES_LEN: usize = 2048;

/// restrictions on a component tree
#[derive(Debug, Clone)]
pub struct Limits {
    /// maximum number of components in the root
    pub components_root: usize,

    /// maximum number of children in a container component (eg. section, container, form)
    pub components_container: usize,

    /// maximum number of components in a component tree
    pub components_total: usize,

    /// maximum depth of a component tree
    pub components_depth: usize,

    /// maximum length of text for labels (eg. buttons)
    pub text_label: usize,

    /// maximum length of text for label descriptions
    pub text_description: usize,

    /// maximum length of text for a text component
    pub text_component: usize,

    /// maximum length of text in a component tree
    pub text_total: usize,

    /// maximum length of a custom id
    pub custom_id: usize,

    /// maximum number of components in a row
    pub row_components: usize,

    /// maximum number of items in a gallery
    pub gallery_items: usize,

    /// maximum number of media items in a component tree
    pub media_items: usize,

    /// maximum number of choices in a select or checkboxes
    pub choices: usize,

    /// maximum length of a regex pattern
    pub regex: usize,

    /// whether this tree can contain interactive components
    pub allow_interactive: bool,
}

impl Default for Limits {
    fn default() -> Self {
        Self::default_inert()
    }
}

/// where a component is placed in the tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Root,
    Container,
    Section,
    DetailsSummary,
    DetailsBody,
    Form,
    Row,
}

#[derive(Debug)]
pub struct ParseOutput {
    pub invalid: Components,
    pub errors: Vec<ParseField>,
}

#[derive(Debug, Clone)]
pub struct ParseField {
    /// path to this field inside the component object
    pub key: Vec<String>,

    /// the errors that occured
    pub errors: Vec<ParseError>,
}

/// an error that occured while parsing/validating components
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("at least one root component is required")]
    MissingRoot,

    #[error("component id {0:?} already used")]
    DuplicateId(ComponentId),

    #[error("component id {0:?} is referenced more than once")]
    DuplicateParent(ComponentId),

    #[error("component id {0:?} is part of a cycle")]
    Cycle(ComponentId),

    #[error("component tree is deeper than {max} levels")]
    DepthExceeded { max: usize },

    #[error("component custom id {0:?} already used")]
    DuplicateCustomId(ComponentCustomId),

    #[error("component {0:?} doesnt exist")]
    UnknownComponent(ComponentId),

    #[error("too many root components (max {max})")]
    TooManyRoots { max: usize },

    #[error("too many components (max {max})")]
    TooManyComponents { max: usize },

    #[error("too many child components (max {max})")]
    TooManyChildren { max: usize },

    #[error("too many choices (max {max})")]
    TooManyChoices { max: usize },

    #[error("too many media items (max {max})")]
    TooManyMedia { max: usize },

    #[error("media id {0:?} already used")]
    DuplicateMedia(MediaId),

    #[error("label text must be between {min} and {max} characters")]
    LabelTextLength { min: u64, max: u64 },

    #[error("label description must be between {min} and {max} characters")]
    LabelDescriptionLength { min: u64, max: u64 },

    #[error("custom_id must be between {min} and {max} characters")]
    CustomIdLength { min: u64, max: u64 },

    #[error("text content too long (max {max})")]
    TextLength { max: u64 },

    #[error("all text in the component tree is too long (max {max})")]
    TextTotalLength { max: u64 },

    #[error("items must be between {min} and {max}")]
    GalleryItemsLength { min: u64, max: u64 },

    #[error("`{child}` cannot be placed in a {parent}")]
    InvalidPlacement {
        parent: &'static str,
        child: &'static str,
    },

    #[error("details must have at least one summary component")]
    MissingSummary,

    #[error("forms cannot be nested")]
    FormNested,

    #[error("a dynamic dataset cannot be used inside a form")]
    DynamicDatasetInForm,

    #[error("`required` is only allowed on inputs inside a form")]
    RequiredOutsideForm,

    #[error("`Submit` buttons must be inside a form")]
    SubmitOutsideForm,

    #[error("`{field}` is not allowed on {component}")]
    FieldNotAllowed {
        field: &'static str,
        component: &'static str,
    },

    #[error("`{field}` minimum must not exceed its maximum")]
    InvalidRange { field: &'static str },

    #[error("invalid format: {reason}")]
    InvalidFormat { reason: &'static str },

    #[error("invalid regex pattern")]
    InvalidRegex,

    #[error("{max} selections requested but only {available} choices exist")]
    TooManySelected { max: u8, available: usize },

    #[error("duplicate entry in allow")]
    DuplicateAllowEntry,

    #[error("url scheme must be http or https")]
    InvalidUrl,

    #[error("too many metadata keys (max {max})")]
    MetadataKeys { max: usize },

    #[error("metadata key must be between 1 and {max} characters")]
    MetadataKeyLength { max: usize },

    #[error("metadata value is too long (max {max})")]
    MetadataValueLength { max: usize },

    #[error("all metadata values combined are too long (max {max})")]
    MetadataValuesLength { max: usize },

    #[error("interactive components are not allowed here")]
    InteractiveNotAllowed,

    #[error("templates are not supported yet")]
    TemplateNotSupported,
}

pub struct ValidationState {
    path: Vec<String>,
    errors: Vec<ParseField>,
    limits: Limits,
    /// ids of components in this tree
    known_ids: HashSet<ComponentId>,
    /// ids of components in the previous version of this tree
    previous_ids: HashSet<ComponentId>,
    /// child id -> the component that references it
    parents: HashMap<ComponentId, Option<ComponentId>>,
    /// components currently being traversed (for cycle detection)
    visiting: HashSet<ComponentId>,
    /// components that have been traversed
    visited: HashSet<ComponentId>,
    seen_custom_ids: Vec<ComponentCustomId>,
    text_total: usize,
    form_depth: usize,
}

impl ValidationState {
    fn new(limits: Limits) -> Self {
        Self {
            path: vec![],
            errors: vec![],
            limits,
            known_ids: HashSet::new(),
            previous_ids: HashSet::new(),
            parents: HashMap::new(),
            visiting: HashSet::new(),
            visited: HashSet::new(),
            seen_custom_ids: vec![],
            text_total: 0,
            form_depth: 0,
        }
    }

    fn with_previous(limits: Limits, previous_ids: HashSet<ComponentId>) -> Self {
        Self {
            previous_ids,
            ..Self::new(limits)
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

    /// push an error for a component, keyed by its id
    fn push_component_error(&mut self, id: ComponentId, error: ParseError) {
        self.enter(format!("component[{}]", id.0), |s| s.push_error(error));
    }

    fn validate_component(&mut self, component: &Component) {
        if !self.limits.allow_interactive && component.ty.is_interactive() {
            self.push_error(ParseError::InteractiveNotAllowed);
        }

        if let Some(allow) = &component.allow {
            self.enter("allow", |s| s.validate_allow(allow));
        }

        self.validate_ty(&component.ty);
    }

    fn validate_ty(&mut self, ty: &ComponentType) {
        match ty {
            ComponentType::Container(_)
            | ComponentType::Section(_)
            | ComponentType::Divider(_)
            | ComponentType::Details(_)
            | ComponentType::Row(_) => {}

            ComponentType::Form(form) => {
                self.enter("custom_id", |s| s.validate_custom_id(&form.custom_id));
            }

            ComponentType::Text(text) => {
                let max = self.limits.text_component;
                self.enter("content", |s| s.validate_text(&text.content, max));
            }

            ComponentType::Media(media) => {
                self.enter("item", |s| s.validate_media(&media.item));
            }

            ComponentType::Gallery(gallery) => {
                let max = self.limits.gallery_items;
                self.enter("items", |s| {
                    if gallery.items.is_empty() || gallery.items.len() > max {
                        s.push_error(ParseError::GalleryItemsLength {
                            min: 1,
                            max: max as u64,
                        });
                    }
                    for (i, item) in gallery.items.iter().enumerate() {
                        s.enter_index(i, |s| s.validate_media(item));
                    }
                });
            }

            ComponentType::Button(button) => self.validate_button(button),

            ComponentType::Input(input) => {
                self.enter("custom_id", |s| s.validate_custom_id(&input.custom_id));
                self.enter("label", |s| s.validate_label(&input.label));
                if let Some(value) = &input.value {
                    let max = self.limits.text_component;
                    self.enter("value", |s| s.validate_text(value, max));
                }
                if let Some(placeholder) = &input.placeholder {
                    let max = self.limits.text_component;
                    self.enter("placeholder", |s| s.validate_text(placeholder, max));
                }
                self.enter("validation", |s| {
                    s.validate_validation(ty, &input.validation, None)
                });
            }

            ComponentType::Textarea(textarea) => {
                self.enter("custom_id", |s| s.validate_custom_id(&textarea.custom_id));
                self.enter("label", |s| s.validate_label(&textarea.label));
                if let Some(value) = &textarea.value {
                    let max = self.limits.text_component;
                    self.enter("value", |s| s.validate_text(value, max));
                }
                if let Some(placeholder) = &textarea.placeholder {
                    let max = self.limits.text_component;
                    self.enter("placeholder", |s| s.validate_text(placeholder, max));
                }
                self.enter("validation", |s| {
                    s.validate_validation(ty, &textarea.validation, None)
                });
            }

            ComponentType::Select(select) => {
                self.enter("custom_id", |s| s.validate_custom_id(&select.custom_id));
                self.enter("label", |s| s.validate_label(&select.label));
                if let Some(placeholder) = &select.placeholder {
                    let max = self.limits.text_component;
                    self.enter("placeholder", |s| s.validate_text(placeholder, max));
                }
                let choices = match &select.dataset {
                    Dataset::Static { choices } => Some(choices.len()),
                    _ => None,
                };
                self.enter("dataset", |s| s.validate_dataset(&select.dataset));
                self.enter("validation", |s| {
                    s.validate_validation(ty, &select.validation, choices)
                });
            }

            ComponentType::Upload(upload) => {
                self.enter("custom_id", |s| s.validate_custom_id(&upload.custom_id));
                self.enter("label", |s| s.validate_label(&upload.label));
                self.enter("validation", |s| {
                    s.validate_validation(ty, &upload.validation, None)
                });
            }

            ComponentType::Checkbox(checkbox) => {
                self.enter("custom_id", |s| s.validate_custom_id(&checkbox.custom_id));
                self.enter("choice", |s| s.validate_choice(&checkbox.choice));
                self.enter("validation", |s| {
                    s.validate_validation(ty, &checkbox.validation, None)
                });
            }

            ComponentType::Checkboxes(checkboxes) => {
                self.enter("custom_id", |s| s.validate_custom_id(&checkboxes.custom_id));
                self.enter("label", |s| s.validate_label(&checkboxes.label));
                self.enter("choices", |s| s.validate_choices(&checkboxes.choices));
                let count = checkboxes.choices.len();
                self.enter("validation", |s| {
                    s.validate_validation(ty, &checkboxes.validation, Some(count))
                });
            }

            ComponentType::Reference(reference) => {
                if !self.previous_ids.contains(&reference.reference_id) {
                    self.push_error(ParseError::UnknownComponent(reference.reference_id));
                }
            }

            ComponentType::Template(_) => {
                self.push_error(ParseError::TemplateNotSupported);
            }
        }
    }

    fn validate_button(&mut self, button: &Button) {
        self.enter("label", |s| s.validate_label(&button.label));
        match &button.action {
            ButtonAction::Open { url } => {
                if !matches!(url.scheme(), "http" | "https") {
                    self.enter("action", |s| s.push_error(ParseError::InvalidUrl));
                }
            }
            ButtonAction::Navigate { .. } => {}
            ButtonAction::Copy { text } => {
                let max = self.limits.text_component;
                self.enter("action", |s| s.validate_text(text, max));
            }
            ButtonAction::Interaction { custom_id } => {
                self.enter("action", |s| s.validate_custom_id(custom_id));
            }
            ButtonAction::Submit => {}
        }
    }

    fn validate_label(&mut self, label: &Label) {
        let max_text = self.limits.text_label;
        let max_description = self.limits.text_description;

        let text_len = label.text.chars().count();
        self.text_total += text_len;
        if text_len == 0 || text_len > max_text {
            self.push_error(ParseError::LabelTextLength {
                min: 1,
                max: max_text as u64,
            });
        }

        if let Some(description) = &label.description {
            let description_len = description.chars().count();
            self.text_total += description_len;
            if description_len == 0 || description_len > max_description {
                self.push_error(ParseError::LabelDescriptionLength {
                    min: 1,
                    max: max_description as u64,
                });
            }
        }
    }

    fn validate_custom_id(&mut self, custom_id: &ComponentCustomId) {
        let len = custom_id.0.chars().count();
        let max = self.limits.custom_id;
        if len == 0 || len > max {
            self.push_error(ParseError::CustomIdLength {
                min: 1,
                max: max as u64,
            });
        }

        if self.seen_custom_ids.contains(custom_id) {
            self.push_error(ParseError::DuplicateCustomId(custom_id.clone()));
        } else {
            self.seen_custom_ids.push(custom_id.clone());
        }
    }

    fn validate_text(&mut self, text: &str, max: usize) {
        let len = text.chars().count();
        self.text_total += len;
        if len > max {
            self.push_error(ParseError::TextLength { max: max as u64 });
        }
    }

    fn validate_choice(&mut self, choice: &Choice) {
        self.enter("label", |s| s.validate_label(&choice.label));
        self.enter("value", |s| s.validate_custom_id(&choice.value));
    }

    fn validate_choices(&mut self, choices: &[Choice]) {
        let max = self.limits.choices;
        if choices.len() > max {
            self.push_error(ParseError::TooManyChoices { max });
        }
        for (i, choice) in choices.iter().enumerate() {
            self.enter_index(i, |s| s.validate_choice(choice));
        }
    }

    fn validate_dataset(&mut self, dataset: &Dataset) {
        if let Dataset::Static { choices } = dataset {
            self.enter("choices", |s| s.validate_choices(choices));
        }
    }

    fn validate_media(&mut self, media: &ComponentMedia) {
        if let Some(description) = &media.description {
            let max = self.limits.text_description;
            self.enter("description", |s| s.validate_text(description, max));
        }

        if let MediaReference::Url { source_url } = &media.media_ref {
            if !matches!(source_url.scheme(), "http" | "https") {
                self.enter("source_url", |s| s.push_error(ParseError::InvalidUrl));
            }
        }
    }

    fn validate_validation(
        &mut self,
        ty: &ComponentType,
        validation: &Validation,
        choices: Option<usize>,
    ) {
        let component = type_name(ty);
        let (allow_length, allow_values, allow_format) = match ty {
            ComponentType::Input(_) => (true, false, true),
            ComponentType::Textarea(_) => (true, false, false),
            ComponentType::Upload(_) => (true, true, false),
            ComponentType::Select(_) | ComponentType::Checkboxes(_) => (false, true, false),
            ComponentType::Checkbox(_) => (false, false, false),
            _ => (false, false, false),
        };

        if !allow_length {
            if validation.min_length.is_some() {
                self.push_error(ParseError::FieldNotAllowed {
                    field: "min_length",
                    component,
                });
            }
            if validation.max_length.is_some() {
                self.push_error(ParseError::FieldNotAllowed {
                    field: "max_length",
                    component,
                });
            }
        }

        if !allow_values {
            if validation.min_values.is_some() {
                self.push_error(ParseError::FieldNotAllowed {
                    field: "min_values",
                    component,
                });
            }
            if validation.max_values.is_some() {
                self.push_error(ParseError::FieldNotAllowed {
                    field: "max_values",
                    component,
                });
            }
        }

        if !allow_format && validation.format.is_some() {
            self.push_error(ParseError::FieldNotAllowed {
                field: "format",
                component,
            });
        }

        if let (Some(min), Some(max)) = (validation.min_length, validation.max_length) {
            if min > max {
                self.push_error(ParseError::InvalidRange {
                    field: "min_length",
                });
            }
        }

        if let (Some(min), Some(max)) = (validation.min_values, validation.max_values) {
            if min > max {
                self.push_error(ParseError::InvalidRange {
                    field: "min_values",
                });
            }
        }

        if let (Some(max), Some(available)) = (validation.max_values, choices) {
            if available > 0 && max as usize > available {
                self.push_error(ParseError::TooManySelected { max, available });
            }
        }

        if let Some(format) = &validation.format {
            self.enter("format", |s| s.validate_format(format));
        }
    }

    fn validate_format(&mut self, format: &InputFormat) {
        match format {
            InputFormat::Numeric { min, max } => {
                if let (Some(min), Some(max)) = (min, max) {
                    if min > max {
                        self.push_error(ParseError::InvalidRange { field: "format" });
                    }
                }
            }
            InputFormat::Integer { min, max } => {
                if let (Some(min), Some(max)) = (min, max) {
                    if min > max {
                        self.push_error(ParseError::InvalidRange { field: "format" });
                    }
                }
            }
            InputFormat::Url => {}
            InputFormat::Regex {
                regex,
                error_message,
            } => {
                let max = self.limits.regex;
                if regex.chars().count() > max {
                    self.push_error(ParseError::TextLength { max: max as u64 });
                }
                #[cfg(feature = "regex_validation")]
                if Regex::new(regex).is_err() {
                    self.push_error(ParseError::InvalidRegex);
                }
                if let Some(error_message) = error_message {
                    let max = self.limits.text_label;
                    self.enter("error_message", |s| s.validate_text(error_message, max));
                }
            }
            InputFormat::Time { date, time } => {
                if !date && !time {
                    self.push_error(ParseError::InvalidFormat {
                        reason: "time format must allow a date, a time, or both",
                    });
                }
            }
        }
    }

    fn validate_allow(&mut self, allow: &Allow) {
        let mut user_ids = HashSet::new();
        for user_id in &allow.user_ids {
            if !user_ids.insert(user_id.clone()) {
                self.push_error(ParseError::DuplicateAllowEntry);
            }
        }

        let mut role_ids = HashSet::new();
        for role_id in &allow.role_ids {
            if !role_ids.insert(role_id.clone()) {
                self.push_error(ParseError::DuplicateAllowEntry);
            }
        }

        let mut permissions: Vec<_> = vec![];
        for permission in &allow.permissions {
            if permissions.contains(&permission) {
                self.push_error(ParseError::DuplicateAllowEntry);
            } else {
                permissions.push(permission);
            }
        }
    }

    fn validate_metadata(&mut self, metadata: &Metadata) {
        if metadata.len() > METADATA_MAX_KEYS {
            self.push_error(ParseError::MetadataKeys {
                max: METADATA_MAX_KEYS,
            });
        }

        let mut values_len = 0;
        for (key, value) in metadata.0.iter() {
            let key_len = key.chars().count();
            if key_len == 0 || key_len > METADATA_MAX_KEY_LEN {
                self.push_error(ParseError::MetadataKeyLength {
                    max: METADATA_MAX_KEY_LEN,
                });
            }

            let value_len = value.chars().count();
            values_len += value_len;
            if value_len > METADATA_MAX_VALUE_LEN {
                self.push_error(ParseError::MetadataValueLength {
                    max: METADATA_MAX_VALUE_LEN,
                });
            }
        }

        if values_len > METADATA_MAX_VALUES_LEN {
            self.push_error(ParseError::MetadataValuesLength {
                max: METADATA_MAX_VALUES_LEN,
            });
        }
    }

    fn validate_media_list(&mut self, media: &[ApiMedia]) {
        let max = self.limits.media_items;
        if media.len() > max {
            self.push_error(ParseError::TooManyMedia { max });
        }

        let mut seen = HashSet::new();
        for (i, item) in media.iter().enumerate() {
            let duplicate = !seen.insert(item.id);
            if duplicate {
                self.enter_index(i, |s| s.push_error(ParseError::DuplicateMedia(item.id)));
            }
        }
    }

    fn validate_context(&mut self, ty: &ComponentType) {
        let validation = match ty {
            ComponentType::Input(input) => Some(&input.validation),
            ComponentType::Textarea(textarea) => Some(&textarea.validation),
            ComponentType::Select(select) => Some(&select.validation),
            ComponentType::Upload(upload) => Some(&upload.validation),
            ComponentType::Checkbox(checkbox) => Some(&checkbox.validation),
            ComponentType::Checkboxes(checkboxes) => Some(&checkboxes.validation),
            _ => None,
        };

        if let Some(validation) = validation {
            if validation.required && self.form_depth == 0 {
                self.enter("validation", |s| {
                    s.push_error(ParseError::RequiredOutsideForm)
                });
            }
        }

        if let ComponentType::Select(select) = ty {
            if self.form_depth > 0 && matches!(select.dataset, Dataset::Dynamic) {
                self.enter("dataset", |s| {
                    s.push_error(ParseError::DynamicDatasetInForm)
                });
            }
        }

        if let ComponentType::Button(button) = ty {
            if self.form_depth == 0 && matches!(button.action, ButtonAction::Submit) {
                self.enter("action", |s| s.push_error(ParseError::SubmitOutsideForm));
            }
        }
    }
}

impl Components {
    fn validate_state(&self, state: &mut ValidationState) {
        let limits = state.limits.clone();

        if self.roots.is_empty() {
            state.push_error(ParseError::MissingRoot);
        }
        if self.roots.len() > limits.components_root {
            state.enter("roots", |s| {
                s.push_error(ParseError::TooManyRoots {
                    max: limits.components_root,
                })
            });
        }
        if self.items.len() > limits.components_total {
            state.push_error(ParseError::TooManyComponents {
                max: limits.components_total,
            });
        }

        state.enter("metadata", |s| s.validate_metadata(&self.metadata));
        state.enter("media", |s| s.validate_media_list(&self.media));

        for (i, component) in self.items.iter().enumerate() {
            state.enter("items", |s| {
                s.enter_index(i, |s| {
                    if !s.known_ids.insert(component.id) {
                        s.push_error(ParseError::DuplicateId(component.id));
                    }
                })
            });
        }

        for component in &self.items {
            state.enter(format!("component[{}]", component.id.0), |s| {
                s.validate_component(component)
            });
        }

        for (i, root_id) in self.roots.iter().enumerate() {
            state.enter("roots", |s| {
                s.enter_index(i, |s| self.visit(*root_id, Some(Slot::Root), None, 1, s))
            });
        }

        // unreachable components still must not contain cycles or be too deep
        for component in &self.items {
            if !state.visited.contains(&component.id) {
                self.check_cycles(component.id, 1, state);
            }
        }

        if state.text_total > limits.text_total {
            state.push_error(ParseError::TextTotalLength {
                max: limits.text_total as u64,
            });
        }
    }

    fn visit(
        &self,
        id: ComponentId,
        slot: Option<Slot>,
        parent: Option<ComponentId>,
        depth: usize,
        state: &mut ValidationState,
    ) {
        let Some(component) = self.items.iter().find(|c| c.id == id) else {
            state.push_component_error(id, ParseError::UnknownComponent(id));
            return;
        };

        if state.visiting.contains(&id) {
            state.push_component_error(id, ParseError::Cycle(id));
            return;
        }

        if state.parents.insert(id, parent).is_some() {
            state.push_component_error(id, ParseError::DuplicateParent(id));
        }

        if state.visited.contains(&id) {
            return;
        }

        let max_depth = state.limits.components_depth;
        if depth > max_depth {
            state.push_component_error(id, ParseError::DepthExceeded { max: max_depth });
            return;
        }

        state.visited.insert(id);
        state.visiting.insert(id);

        if let Some(slot) = slot {
            if let Some((parent_name, child_name)) = placement_violation(slot, &component.ty) {
                state.push_component_error(
                    id,
                    ParseError::InvalidPlacement {
                        parent: parent_name,
                        child: child_name,
                    },
                );
            }
        }

        if matches!(component.ty, ComponentType::Form(_)) && state.form_depth > 0 {
            state.push_component_error(id, ParseError::FormNested);
        }

        state.enter(format!("component[{}]", component.id.0), |s| {
            s.validate_context(&component.ty)
        });

        let max_children = state.limits.components_container;
        match &component.ty {
            ComponentType::Container(container) => {
                self.visit_children(
                    &container.components,
                    Slot::Container,
                    id,
                    depth,
                    max_children,
                    state,
                );
            }
            ComponentType::Section(section) => {
                self.visit_children(
                    &section.components,
                    Slot::Section,
                    id,
                    depth,
                    max_children,
                    state,
                );
            }
            ComponentType::Details(details) => {
                if details.summary.is_empty() {
                    state.push_component_error(id, ParseError::MissingSummary);
                }
                self.visit_children(
                    &details.summary,
                    Slot::DetailsSummary,
                    id,
                    depth,
                    max_children,
                    state,
                );
                self.visit_children(
                    &details.children,
                    Slot::DetailsBody,
                    id,
                    depth,
                    max_children,
                    state,
                );
            }
            ComponentType::Form(form) => {
                state.form_depth += 1;
                self.visit_children(&form.components, Slot::Form, id, depth, max_children, state);
                state.form_depth -= 1;
            }
            ComponentType::Row(row) => {
                let max = state.limits.row_components;
                self.visit_children(&row.components, Slot::Row, id, depth, max, state);
            }
            _ => {}
        }

        state.visiting.remove(&id);
    }

    fn visit_children(
        &self,
        ids: &[ComponentId],
        slot: Slot,
        parent: ComponentId,
        depth: usize,
        max: usize,
        state: &mut ValidationState,
    ) {
        if ids.len() > max {
            state.push_component_error(parent, ParseError::TooManyChildren { max });
        }

        for id in ids {
            self.visit(*id, Some(slot), Some(parent), depth + 1, state);
        }
    }

    fn check_cycles(&self, id: ComponentId, depth: usize, state: &mut ValidationState) {
        if state.visited.contains(&id) {
            return;
        }

        let Some(component) = self.items.iter().find(|c| c.id == id) else {
            state.push_component_error(id, ParseError::UnknownComponent(id));
            return;
        };

        if state.visiting.contains(&id) {
            state.push_component_error(id, ParseError::Cycle(id));
            return;
        }

        let max_depth = state.limits.components_depth;
        if depth > max_depth {
            state.push_component_error(id, ParseError::DepthExceeded { max: max_depth });
            return;
        }

        state.visiting.insert(id);
        for child in child_ids(&component.ty) {
            self.check_cycles(child, depth + 1, state);
        }
        state.visiting.remove(&id);
        state.visited.insert(id);
    }
}

impl Component {
    /// get the ids of this component's children
    pub fn child_ids(&self) -> Vec<ComponentId> {
        child_ids(&self.ty)
    }
}

impl Create {
    /// parse and validate
    pub fn parse(self, limits: Limits) -> Result<Components, ParseOutput> {
        self.parse_with_previous(limits, HashSet::new())
    }

    /// parse and validate, allowing [`ComponentType::Reference`] to point at components
    /// from the previous version of this tree
    pub fn parse_with_previous(
        self,
        limits: Limits,
        previous_ids: HashSet<ComponentId>,
    ) -> Result<Components, ParseOutput> {
        let mut state = ValidationState::with_previous(limits, previous_ids);
        self.inner.validate_state(&mut state);

        if state.errors.is_empty() {
            Ok(self.inner)
        } else {
            Err(ParseOutput {
                invalid: self.inner,
                errors: state.errors,
            })
        }
    }
}

impl Components {
    /// validate these components without consuming them
    pub fn validate(&self, limits: &Limits) -> Result<(), Vec<ParseField>> {
        let mut state = ValidationState::new(limits.clone());
        self.validate_state(&mut state);

        if state.errors.is_empty() {
            Ok(())
        } else {
            Err(state.errors)
        }
    }
}

impl Limits {
    pub fn default_inert() -> Self {
        Self {
            components_root: 16,
            components_container: 64,
            components_total: 64,
            components_depth: 32,
            text_label: 64,
            text_description: 2048,
            text_component: 8192,
            text_total: 65535,
            custom_id: 128,
            row_components: 5,
            gallery_items: 20,
            media_items: 20,
            choices: 25,
            regex: 256,
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

fn type_name(ty: &ComponentType) -> &'static str {
    match ty {
        ComponentType::Container(_) => "Container",
        ComponentType::Section(_) => "Section",
        ComponentType::Divider(_) => "Divider",
        ComponentType::Details(_) => "Details",
        ComponentType::Form(_) => "Form",
        ComponentType::Row(_) => "Row",
        ComponentType::Text(_) => "Text",
        ComponentType::Media(_) => "Media",
        ComponentType::Gallery(_) => "Gallery",
        ComponentType::Button(_) => "Button",
        ComponentType::Input(_) => "Input",
        ComponentType::Textarea(_) => "Textarea",
        ComponentType::Select(_) => "Select",
        ComponentType::Upload(_) => "Upload",
        ComponentType::Checkbox(_) => "Checkbox",
        ComponentType::Checkboxes(_) => "Checkboxes",
        ComponentType::Reference(_) => "Reference",
        ComponentType::Template(_) => "Template",
    }
}

fn slot_name(slot: Slot) -> &'static str {
    match slot {
        Slot::Root => "root",
        Slot::Container => "Container",
        Slot::Section => "Section",
        Slot::DetailsSummary => "Details summary",
        Slot::DetailsBody => "Details body",
        Slot::Form => "Form",
        Slot::Row => "Row",
    }
}

fn placement_violation(slot: Slot, ty: &ComponentType) -> Option<(&'static str, &'static str)> {
    let allowed = match slot {
        Slot::Root | Slot::Container | Slot::Section | Slot::DetailsBody | Slot::Form => true,
        Slot::DetailsSummary => matches!(
            ty,
            ComponentType::Text(_) | ComponentType::Media(_) | ComponentType::Reference(_)
        ),
        Slot::Row => matches!(
            ty,
            ComponentType::Button(_) | ComponentType::Text(_) | ComponentType::Reference(_)
        ),
    };

    if allowed {
        None
    } else {
        Some((slot_name(slot), type_name(ty)))
    }
}

fn child_ids(ty: &ComponentType) -> Vec<ComponentId> {
    match ty {
        ComponentType::Container(container) => container.components.clone(),
        ComponentType::Section(section) => section.components.clone(),
        ComponentType::Details(details) => details
            .summary
            .iter()
            .chain(details.children.iter())
            .copied()
            .collect(),
        ComponentType::Form(form) => form.components.clone(),
        ComponentType::Row(row) => row.components.clone(),
        _ => Vec::new(),
    }
}
