#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::misc::Color;
use crate::v2::types::media::{Media, MediaReference};

/// maximum number of components in a tree
pub(crate) const MAX_COMPONENTS: usize = 64;

/// maximum depth of components in a tree
pub(crate) const MAX_DEPTH: usize = 32;

/// maximum length of all text across all components
///
/// - calculates recursively
/// - currently uses bytes (this may be changed later)
/// - `Text` content
/// - `Button` label
/// - `LinkButton` label and url
/// - `Media` and `Gallery` description
/// - `Details` summary and details
pub(crate) const MAX_TOTAL_TEXT_LENGTH: usize = 65535;

/// An identifier for a component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ComponentId(pub u16);

/// A developer-defined identifier for an interactive component.
///
/// Min 1 char, max 128 chars.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ComponentCustomId(pub String);

/// a single component in a tree
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Component<M = Media> {
    pub id: ComponentId,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: ComponentType<Component<M>, M>,
}

/// a request body for creating or updating a component tree
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ComponentCreate<M = MediaReference> {
    // populate with sequential ids server side if None
    pub id: Option<ComponentId>,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: ComponentType<ComponentCreate<M>, M>,
}

// deserialize strings as a Text component without an id.
#[cfg(feature = "serde")]
impl<'de, M: Deserialize<'de>> Deserialize<'de> for ComponentCreate<M> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper<M> {
            Text(String),
            Struct {
                #[serde(default)]
                id: Option<ComponentId>,
                #[serde(flatten)]
                ty: ComponentType<ComponentCreate<M>, M>,
            },
        }

        let helper = Helper::<M>::deserialize(deserializer)?;
        match helper {
            Helper::Text(content) => Ok(ComponentCreate {
                id: None,
                ty: ComponentType::Text { content },
            }),
            Helper::Struct { id, ty } => Ok(ComponentCreate { id, ty }),
        }
    }
}

mod ic {
    pub trait Sealed {}
}

pub trait IsComponent<C, M>: ic::Sealed {
    fn ty(&self) -> &ComponentType<C, M>;
}

impl<M> ic::Sealed for Component<M> {}
impl<M> ic::Sealed for ComponentCreate<M> {}

impl<M> IsComponent<Component<M>, M> for Component<M> {
    fn ty(&self) -> &ComponentType<Component<M>, M> {
        &self.ty
    }
}

impl<M> IsComponent<ComponentCreate<M>, M> for ComponentCreate<M> {
    fn ty(&self) -> &ComponentType<ComponentCreate<M>, M> {
        &self.ty
    }
}

/// components
///
/// ## layout
///
/// - `Container` creates a visually distinct section
/// - `Section` creates a section without any margin/padding
/// - `Details` creates a collapseable section
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
/// - `LinkButton` is a link that looks like a button
///
/// ## logic
///
/// - `Reference` move or clone another component
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ComponentType<C, M> {
    /// a clickable button
    Button {
        label: String,
        style: ButtonStyle,

        /// developer-defined identifier for this component
        custom_id: ComponentCustomId,
        // TODO: disabled, emoji
    },

    /// a link that looks like a button
    LinkButton {
        label: String,

        /// what to link to
        url: Option<Url>,
        // TODO: disabled, emoji
    },

    /// a group of other components
    Container {
        components: Vec<C>,
        color: Option<Color>,
    },

    /// markdown text
    // maybe rename to Markdown?
    Text { content: String },

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

    /// a collapsible section
    Details {
        #[cfg_attr(feature = "serde", serde(default))]
        open: bool,
        color: Option<Color>,
        summary: Vec<C>,
        details: Vec<C>,
    },

    /// a section without any margin/padding
    Section {
        color: Option<Color>,
        components: Vec<C>,
    },

    /// display media
    ///
    /// min 1 max 20 items
    Media { items: Vec<ComponentMedia<M>> },

    /// display a carousel of media
    ///
    /// min 1 max 20 items
    Gallery { items: Vec<ComponentMedia<M>> },
}

impl<M> Component<M> {
    /// Find a component by its ID recursively.
    pub fn find_by_id(&self, id: ComponentId) -> Option<&Component<M>> {
        if self.id == id {
            return Some(self);
        }
        match &self.ty {
            ComponentType::Container { components, .. }
            | ComponentType::Section { components, .. } => {
                components.iter().find_map(|c| c.find_by_id(id))
            }
            ComponentType::Details {
                summary, details, ..
            } => summary
                .iter()
                .find_map(|c| c.find_by_id(id))
                .or_else(|| details.iter().find_map(|c| c.find_by_id(id))),
            _ => None,
        }
    }

    /// Visit all component IDs in the tree recursively.
    pub fn visit_ids<F>(&self, f: &mut F)
    where
        F: FnMut(ComponentId),
    {
        f(self.id);
        match &self.ty {
            ComponentType::Container { components, .. }
            | ComponentType::Section { components, .. } => {
                for c in components {
                    c.visit_ids(f);
                }
            }
            ComponentType::Details {
                summary, details, ..
            } => {
                for c in summary {
                    c.visit_ids(f);
                }
                for c in details {
                    c.visit_ids(f);
                }
            }
            _ => {}
        }
    }
}

impl<M: Clone> Component<M> {
    /// Extract all media from the component tree.
    pub fn media_ids(&self) -> Vec<M> {
        let mut ids = vec![];
        self.collect_media(&mut ids);
        ids
    }

    /// Recursively collect all media from the component tree.
    fn collect_media(&self, ids: &mut Vec<M>) {
        match &self.ty {
            ComponentType::Container { components, .. }
            | ComponentType::Section { components, .. } => {
                for c in components {
                    c.collect_media(ids);
                }
            }
            ComponentType::Details {
                summary, details, ..
            } => {
                for c in summary {
                    c.collect_media(ids);
                }
                for c in details {
                    c.collect_media(ids);
                }
            }
            ComponentType::Media { items } | ComponentType::Gallery { items } => {
                for item in items {
                    ids.push(item.media.clone());
                }
            }
            ComponentType::Button { .. }
            | ComponentType::LinkButton { .. }
            | ComponentType::Text { .. }
            | ComponentType::Reference { .. } => {}
        }
    }
}

/// media to display in a component
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ComponentMedia<M> {
    pub media: M,

    /// description for this media
    ///
    /// min 1 max 1024 chars
    pub description: Option<String>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub spoiler: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ButtonStyle {
    #[default]
    Primary,
    Secondary,
    Danger,
}
