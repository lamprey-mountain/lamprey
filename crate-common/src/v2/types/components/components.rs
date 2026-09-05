use lamprey_macros::record;
use url::Url;

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
        media::{Media, MediaReference},
    },
};

/// top-level container for components
#[record]
#[derive(Default)]
pub struct Components {
    /// the ids of top level components
    pub roots: Vec<ComponentId>,

    /// list of components
    pub items: Vec<Component>,

    /// media referenced in the components
    // NOTE: should i remove this and send a vec of all media in the top level response instead?
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media: Vec<Media>,

    /// application-specific metadata
    // NOTE: maybe rename to `variables`? or have both variables to use with templates later and generic metadata?
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

// NOTE: can components be reused? maybe separate ComponentType (and maybe
// Allow) from the rest of the component?

/// a single component in a tree
#[record]
pub struct Component {
    pub id: ComponentId,

    #[serde(flatten)]
    pub ty: ComponentType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Allow>,
}

/// a piece of media used in a component
#[record]
pub struct ComponentMedia {
    /// what media is being referenced
    ///
    /// clients can use any value, server will always send [`MediaReference::Media`]
    #[serde(flatten)]
    pub media_ref: MediaReference,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "is_false")]
    pub spoiler: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl ComponentMedia {
    pub fn new_media(media_id: MediaId) -> Self {
        Self {
            media_ref: MediaReference::Media { media_id },
            description: None,
            spoiler: false,
        }
    }

    pub fn new_url(source_url: Url) -> Self {
        Self {
            media_ref: MediaReference::Url { source_url },
            description: None,
            spoiler: false,
        }
    }

    #[inline]
    pub fn media_id(&self) -> Option<MediaId> {
        self.media_ref.media_id()
    }
}

// TODO: impl?
// impl ComponentMediaBuilder {
//     pub fn description(mut self, description: impl Into<String>) -> Self {
//         self.description = Some(description.into());
//         self
//     }
//
//     pub fn spoiler(mut self) -> Self {
//         self.spoiler = true;
//         self
//     }
// }
//
// impl ComponentBuilder {
//     pub fn new(ty: ComponentType) -> Self {
//         Self {
//             id: None,
//             ty,
//             allow: None,
//         }
//     }
//
//     /// set the id for this component
//     pub fn id(mut self, id: impl Into<ComponentId>) -> Self {
//         self.id = Some(id.into());
//         self
//     }
//
//     /// set the access control for interacting with this component
//     pub fn allow(mut self, allow: impl Into<Allow>) -> Self {
//         self.allow = Some(allow.into());
//         self
//     }
// }

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
// TODO: more components? some seem a bit advanced though
// - `Root` pseudo component for the component root
// - `Show` conditionally render some components depending on variables
// - `For` render a list of components depending on variables
//
// TODO(?): maybe add Column, Grid, etc for more layout?
// TODO(?): maybe merge Checkbox and Checkboxes
// TODO(?): maybe add multiple root components, eg. Modal, Sidebar, Message
#[record]
#[serde(tag = "type")]
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
        #[schema(no_recursion)]
        components: Vec<ComponentId>,
        color: Option<Color>,
    },

    /// markdown text
    // maybe rename to Markdown?
    Text { content: String },

    /// a collapsible section
    Details {
        #[serde(default)]
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

    /// display a single piece of media
    Media { item: ComponentMedia },

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
        // TODO: implement this
        // template_id: ComponentTemplateId,
    },
}

// NOTE: in the past, components formed a json tree. now they're flat. maybe its still nice to be able to create components using a tree?

// NOTE: in the past, when creating components, a String could be used as a shorthand to create a Text component with no content.
// since every component now is required to have an id and the structure is flat, i don't think this is possible? but having shortcuts to save bandwidth would be nice.

#[record]
#[serde(transparent)]
pub struct ComponentsCreate {
    inner: Components,
}

impl ComponentsCreate {
    pub fn new(c: Components) -> Self {
        Self { inner: c }
    }

    /// get a reference to the underlying [`Components`] without validating
    pub fn get_unvalidated(&self) -> &Components {
        &self.inner
    }

    /// get a mutable reference to the underlying [`Components`] without validating
    pub fn get_unvalidated_mut(&mut self) -> &mut Components {
        &mut self.inner
    }

    /// get the underlying [`Components`] without validating
    pub fn unwrap_unvalidated(self) -> Components {
        self.inner
    }
}
