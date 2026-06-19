#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v2::types::components::ComponentCustomId;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ButtonStyle {
    #[default]
    Primary,
    Secondary,
    Danger,
    // TODO: more styles?
}

/// a label for interactive components
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
// TODO: impl From<String> for Label
// TODO: impl Label pub fn new(text: impl Into<String>, description: ???)
pub struct Label {
    /// the label text
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub text: String,

    /// the label description
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 2048))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    pub description: Option<String>,
}

/// valiation for an interactive component
// TODO: add utoipa/validator attrs (eg. min_length must be None or Some > 1
// TODO: validation for Validation
// TODO: split struct apart into granular per input type structs
// TODO: restrict mime types for Upload
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Validation {
    /// mark this input as required
    ///
    /// allowed for all inputs inside `Form`
    pub required: bool,

    /// the minimum allowed length of text
    ///
    /// allowed for `Input`, `Textarea`, `Upload` (as file size)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub min_length: Option<u32>,

    /// the maximum allowed length of text
    ///
    /// allowed for `Input`, `Textarea`, `Upload` (as file size)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub max_length: Option<u32>,

    /// the minimum number of selected options
    ///
    /// ui: setting this to >1 will mark this option as required
    ///
    /// allowed for `Select`, `Checkboxes`, `Upload` (as file count)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub min_values: Option<u8>,

    /// the maximum number of selected options
    ///
    /// ui: setting this to =1 in `Checkboxes` will style and behave like radio buttons
    ///
    /// allowed for `Select`, `Checkboxes`, `Upload` (as file count)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub max_values: Option<u8>,

    /// the required input format
    ///
    /// allowed for `Input`
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub format: Option<InputFormat>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum InputFormat {
    /// must be a number
    Number {
        min: Option<f64>,
        max: Option<f64>,

        #[cfg_attr(feature = "serde", serde(default))]
        integer: bool,
    },

    /// must be a url
    Url,

    /// must match this regex
    Regex {
        /// the regex to use
        ///
        /// uses rust's regex format
        regex: String,

        /// custom error message
        ///
        /// ui: will be displayed on failure
        error_message: Option<String>,
    },

    /// must be a time
    ///
    /// ui: renders as a date/time picker in ui
    Time {
        /// whether to only select a date, rather than an exact time
        #[cfg_attr(feature = "serde", serde(default))]
        date_only: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum TextareaStyle {
    /// normal textarea style
    Default,

    /// supports markdown
    Markdown,
}

/// where to pull options for a `Select` from
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SelectDataset {
    /// a static set of options
    Static { options: Vec<SelectOption> },

    /// dynamically provided from the application
    Dynamic,

    // platform-provided data
    /// a user
    ///
    /// lists all users able to view the current channel
    // TODO: with_roles
    User,

    /// a role
    ///
    /// lists all roles in the current room
    Role,

    /// a channel
    ///
    /// lists all channels visible to the current user
    // TODO: types, parent_id
    Channel,

    /// a user or role
    ///
    /// lists the options from `User` and `Role` in one list
    // TODO: with_roles
    Mentionable,
}

/// a selectable option
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SelectOption {
    /// this option's label
    pub label: Label,

    /// custom id for tis option
    pub value: ComponentCustomId,

    /// whether to select this option by default
    #[cfg_attr(feature = "serde", serde(default))]
    pub default: bool,
}

// interaction response type: input validation failed
// interaction response type: dynamic select options
