//! types for each component type

use lamprey_macros::record;

use crate::{
    v1::types::misc::Color,
    v2::types::components::{
        ComponentCustomId, ComponentId, ComponentMedia,
        interactive::{
            ButtonAction, ButtonStyle, Choice, Dataset, Label, TextareaStyle, Validation,
        },
    },
};

/// the type of a component
///
/// ## layout
///
/// - `Container` creates a visually distinct group of other components
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
// NOTE: in the future i could *maybe* add a checkbox grid and/or a linear scale/rating input
#[record]
#[serde(tag = "type")]
pub enum ComponentType {
    // layout
    Container(Container),
    Section(Section),
    Details(Details),
    Form(Form),
    Row(Row),

    // content
    Text(Text),
    Media(Media),
    Gallery(Gallery),

    // interactive
    Button(Button),
    Input(Input),
    Textarea(Textarea),
    Select(Select),
    Upload(Upload),
    Checkbox(Checkbox),
    Checkboxes(Checkboxes),

    // logic
    Reference(Reference),
    Template(Template),
}

/// a single line text input
#[record]
pub struct Input {
    pub custom_id: ComponentCustomId,
    pub label: Label,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,

    pub validation: Validation,
}

/// a multiline line text input
#[record]
pub struct Textarea {
    pub custom_id: ComponentCustomId,
    pub label: Label,
    pub style: TextareaStyle,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,

    pub validation: Validation,
}

/// creates a dropdown select menu
///
/// creates an interaction on select outside of a `Form`, waits for submit otherwise
#[record]
pub struct Select {
    pub custom_id: ComponentCustomId,
    pub label: Label,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,

    pub dataset: Dataset,
    pub validation: Validation,
}

/// creates a file upload area
#[record]
pub struct Upload {
    pub custom_id: ComponentCustomId,
    pub label: Label,
    pub validation: Validation,
}

/// creates a single checkbox
///
/// use the label from `option`
#[record]
pub struct Checkbox {
    pub custom_id: ComponentCustomId,
    pub choice: Choice,
    pub validation: Validation,
}

/// creates a set of checkboxes
#[record]
pub struct Checkboxes {
    pub custom_id: ComponentCustomId,
    pub label: Label,
    pub choices: Vec<Choice>,
    pub validation: Validation,
}

/// markdown text
// NOTE: maybe rename to Markdown?
#[record]
pub struct Text {
    pub content: String,
}

/// a collapsible section
#[record]
pub struct Details {
    #[serde(default)]
    pub open: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// the first child is the summary
    pub children: Vec<ComponentId>,
}

/// a section without any margin/padding
#[record]
pub struct Section {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    pub components: Vec<ComponentId>,
}

/// a semantic grouping of input elements that can be submitted together
///
/// forms cannot be nested
#[record]
pub struct Form {
    pub custom_id: ComponentCustomId,
    pub components: Vec<ComponentId>,
}

/// a horizontal group of components
///
/// intended for rows of buttons. cannot hold any component type other than `Button`. maximum of 5 components per row.
#[record]
pub struct Row {
    // TODO: maybe allow other component types?
    pub components: Vec<ComponentId>,
}

/// display a single piece of media
#[record]
pub struct Media {
    pub item: ComponentMedia,
}

/// display a carousel of media
///
/// min 1 max 20 items
#[record]
pub struct Gallery {
    pub items: Vec<ComponentMedia>,
}

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
#[record]
pub struct Reference {
    pub reference_id: ComponentId,
}

/// reuse a template
#[record]
pub struct Template {
    // TODO: implement this
    // template_id: ComponentTemplateId,
}

/// a clickable button
#[record]
pub struct Button {
    pub label: Label,
    pub style: ButtonStyle,

    /// what to do when this button is clicked
    pub action: ButtonAction,
}

/// a group of other components
#[record]
pub struct Container {
    /// the child components in this container
    // NOTE: this may be empty. should i allow empty containers in flumes only or always?
    // TODO: use utoipa/validate attrs. maybe add to record macro.
    // #[schema(max_length = 64)]
    // #[validate(length(max = 64))]
    pub components: Vec<ComponentId>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

impl ComponentType {
    // pub fn is_interactive(&self) -> bool;
    // has_children
    // children
}

impl Row {
    // pub fn can_hold(&self, c: &ComponentType) -> bool;
}

// TODO: maybe add trait ComponentType {}?
// TODO: maybe add trait ParentComponent { fn children(); fn can_hold(); ... }
