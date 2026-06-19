#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{
    v1::types::{metadata::Metadata, misc::Color},
    v2::types::{
        MediaId,
        components::{
            ComponentCustomId, ComponentId,
            acl::Allow,
            action::ButtonAction,
            interactive::{
                ButtonStyle, Label, SelectDataset, SelectOption, TextareaStyle, Validation,
            },
        },
        media::Media,
    },
};

/// top-level container for components
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Components {
    /// the ids of top level components
    pub roots: Vec<ComponentId>,

    /// list of components
    pub items: Vec<Component>,

    /// media referenced in the components
    pub media: Vec<Media>,

    /// application-specific metadata
    // NOTE: maybe rename to `variables`?
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Metadata::is_empty"))]
    pub metadata: Metadata,
}

// NOTE: can components be reused? maybe separate ComponentType (and maybe
// Allow) from the rest of the component?

/// a single component in a tree
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Component {
    pub id: ComponentId,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: ComponentType,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub allow: Option<Allow>,
}

/// a piece of media used in a component
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ComponentMedia {
    pub media_id: MediaId,
    pub description: Option<String>,
    pub spoiler: bool,
}

/// the type of a component
///
/// ## layout
///
/// - `Container` creates a visually distinct section
/// - `Section` creates a section without any margin/padding
/// - `Details` creates a collapseable section
/// - `Form` creates a form that can be filled out and submitted
/// - `Row` creates a container that arranges components horizontally
///
/// ## content
///
/// - `Text` displays markdown text
/// - `Media` display a single piece of media
/// - `Gallery` display multiple media
///
/// ## interactivity
///
/// - `Button` is clicky button
/// - `Input` creates a single line text input
/// - `Textarea` creates a multiline text input
/// - `Select` creates a dropdown select menu
/// - `Upload` creates a file upload area
/// - `Checkbox` creates a single checkbox
/// - `Checkboxes` creates a set of checkboxes
///
/// everything besides `Button` must be in a `Form`
///
/// ## logic
///
/// - `Reference` move or clone another component
/// - `Template` use a template
// TODO: Show/For logic components? seems a bit advanced though
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ComponentType {
    /// a clickable button
    Button {
        label: Label,
        style: ButtonStyle,

        /// what to do when this button is clicked
        action: ButtonAction,
    },

    /// a single line text input
    Input {
        custom_id: ComponentCustomId,
        label: Label,
        value: Option<String>,
        placeholder: Option<String>,
        validation: Validation,
    },

    /// a multiline line text input
    Textarea {
        custom_id: ComponentCustomId,
        label: Label,
        style: TextareaStyle,
        value: Option<String>,
        placeholder: Option<String>,
        validation: Validation,
    },

    /// creates a dropdown select menu
    ///
    /// creates an interaction on select outside of a `Form`, waits for submit otherwise
    Select {
        custom_id: ComponentCustomId,
        label: Label,
        placeholder: Option<String>,
        dataset: SelectDataset,
        validation: Validation,
    },

    /// creates a file upload area
    Upload {
        custom_id: ComponentCustomId,
        label: Label,
        validation: Validation,
    },

    /// creates a single checkbox
    ///
    /// use the label from `option`
    Checkbox {
        custom_id: ComponentCustomId,
        option: SelectOption,
        validation: Validation,
    },

    /// creates a set of checkboxes
    Checkboxes {
        custom_id: ComponentCustomId,
        label: Label,
        options: Vec<SelectOption>,
        validation: Validation,
    },

    // NOTE: in the future i could *maybe* add a checkbox grid and/or a linear scale/rating input
    /// a group of other components
    Container {
        #[cfg_attr(feature = "utoipa", schema(no_recursion))]
        components: Vec<ComponentId>,
        color: Option<Color>,
    },

    /// markdown text
    // maybe rename to Markdown?
    Text { content: String },

    /// a collapsible section
    Details {
        #[cfg_attr(feature = "serde", serde(default))]
        open: bool,

        color: Option<Color>,
        summary: Vec<ComponentId>,
        details: Vec<ComponentId>,
    },

    /// a section without any margin/padding
    Section {
        color: Option<Color>,
        components: Vec<ComponentId>,
    },

    /// a semantic grouping of input elements that can be submitted together
    ///
    /// forms cannot be nested
    Form {
        custom_id: ComponentCustomId,
        components: Vec<ComponentId>,
    },

    /// a horizontal group of components
    ///
    /// intended for rows of buttons. cannot hold any component type other than `Button`. maximum of 5 components per row.
    Row { components: Vec<ComponentId> },

    /// display media
    ///
    /// min 1 max 20 items
    Media { items: Vec<ComponentMedia> },

    /// display a carousel of media
    ///
    /// min 1 max 20 items
    Gallery { items: Vec<ComponentMedia> },

    /// reference an existing component from a previous version of this tree.
    ///
    /// if you want to replace most of a component tree, but leave certain components untouched, you can use this
    ///
    /// ## uses
    ///
    /// - **Moving:** To keep an existing component with the same ID, set `ComponentCreate.id`
    ///   to the same value as `reference_id`.
    /// - **Cloning:** To create a deep clone of an existing component, set
    ///   `ComponentCreate.id` to a new ID (or leave it `None`). All children,
    ///   if there are any, are recursively cloned and assigned new ids.
    Reference { reference_id: ComponentId },

    /// reuse a template
    Template {
        // TODO
        // template_id: ComponentTemplateId,
    },
}
