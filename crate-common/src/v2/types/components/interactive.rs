use lamprey_macros::record;

use crate::v2::types::components::ComponentCustomId;

#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    #[default]
    Primary,
    Secondary,
    Danger,
    // TODO: more styles?
}

/// a label for interactive components
#[record]
#[derive(PartialEq, Eq, Hash)]
// TODO: impl Label pub fn new(text: impl Into<String>, description: ???)
pub struct Label {
    /// the label text
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub text: String,

    /// the label description
    #[schema(min_length = 1, max_length = 2048)]
    #[validate(length(min = 1, max = 2048))]
    pub description: Option<String>,
}

/// valiation for an interactive component
// TODO: add utoipa/validator attrs (eg. min_length must be None or Some > 1
// TODO: impl validator::Validate for Validation
// TODO: add some way to restrict mime types for Upload
// TODO(?): split struct apart into granular per input type structs
#[record]
#[derive(Default)]
pub struct Validation {
    /// mark this input as required
    ///
    /// allowed for all inputs inside `Form`
    pub required: bool,

    /// the minimum allowed length of text
    ///
    /// allowed for `Input`, `Textarea`, `Upload` (as file size)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,

    /// the maximum allowed length of text
    ///
    /// allowed for `Input`, `Textarea`, `Upload` (as file size)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,

    /// the minimum number of selected options
    ///
    /// ui: setting this to >1 will mark this option as required
    ///
    /// allowed for `Select`, `Checkboxes`, `Upload` (as file count)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_values: Option<u8>,

    /// the maximum number of selected options
    ///
    /// ui: setting this to =1 in `Checkboxes` will style and behave like radio buttons
    ///
    /// allowed for `Select`, `Checkboxes`, `Upload` (as file count)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_values: Option<u8>,

    /// the required input format
    ///
    /// allowed for `Input`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<InputFormat>,
}

impl Validation {
    pub fn is_empty(&self) -> bool {
        !self.required
            && self.min_length.is_none()
            && self.min_values.is_none()
            && self.max_length.is_none()
            && self.max_values.is_none()
            && self.format.is_none()
    }
}

#[record]
#[serde(tag = "type")]
pub enum InputFormat {
    /// must be a floating point number
    Numeric {
        /// the minimum value, inclusive
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,

        /// the maximum value, inclusive
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
        // TODO(?): add option to make limits exclusive
    },

    /// must be an integer
    Integer {
        /// the minimum value, inclusive
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<i64>,

        /// the maximum value, inclusive
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<i64>,
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
    ///
    /// at least one of `date` or `time` must be true
    Time {
        /// whether to select a date
        #[serde(default = "true_fn")]
        date: bool,

        /// whether to select a time
        #[serde(default = "true_fn")]
        time: bool,
    },
}

fn true_fn() -> bool {
    true
}

#[record]
#[derive(Copy, PartialEq, Eq)]
pub enum TextareaStyle {
    /// normal textarea style
    Default,

    /// supports markdown
    Markdown,
}

/// where to pull options for a `Select` from
#[record]
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
#[record]
pub struct SelectOption {
    /// this option's label
    pub label: Label,

    /// custom id for tis option
    pub value: ComponentCustomId,

    /// whether to select this option by default
    #[serde(default)]
    pub default: bool,
}

// interaction response type: input validation failed
// interaction response type: dynamic select options

impl<S: Into<String>> From<S> for Label {
    fn from(text: S) -> Self {
        Label {
            text: text.into(),
            description: None,
        }
    }
}
