use std::fmt::Debug;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::MediaId;
use crate::v1::types::components::acl::Allow;
use crate::v1::types::e2ee::media::EncryptedMedia;
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

impl std::ops::Deref for ComponentId {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


/// A developer-defined identifier for an interactive component.
///
/// Min 1 char, max 128 chars.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ComponentCustomId(pub String);

/// a single component in a tree
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Component<C: ComponentState> {
    pub id: C::Id,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: ComponentType<C>,

    /// restrict who is allowed to interact with this component
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Allow>,
}

pub type ComponentCreate = Component<Create>;
pub type ComponentCanonical = Component<Canonical>;
pub type ComponentThin = Component<Thin>;
pub type ComponentEncrypted = Component<Encrypted>;

/// top-level container for components
#[derive(Debug, Clone, PartialEq)]
pub struct Components<C: ComponentState> {
    pub inner: Vec<Component<C>>,
}

#[cfg(feature = "serde")]
mod _s {
    use serde::{Deserialize, Serialize};

    use crate::v1::types::components::{
        Canonical, ComponentState, Create, Encrypted, Thin, acl::Allow,
    };

    use super::{Component, ComponentCreate, ComponentId, ComponentType, Components};

    impl<'de> Deserialize<'de> for Component<Create> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            #[derive(Deserialize)]
            #[serde(untagged)]
            enum Helper {
                // deserialize strings as a Text component without an id.
                Text(String),
                Struct {
                    #[serde(default)]
                    id: Option<ComponentId>,

                    #[serde(flatten)]
                    ty: ComponentType<Create>,

                    #[serde(default, skip_serializing_if = "Option::is_none")]
                    allow: Option<Allow>,
                },
            }

            let helper = Helper::deserialize(deserializer)?;
            match helper {
                Helper::Text(content) => Ok(ComponentCreate {
                    id: None,
                    ty: ComponentType::Text { content },
                    allow: None,
                }),
                Helper::Struct { id, ty, allow } => Ok(Component { id, ty, allow }),
            }
        }
    }

    impl<'de> Deserialize<'de> for Component<Canonical> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            #[derive(Deserialize)]
            #[serde(untagged)]
            enum Helper {
                Struct {
                    id: ComponentId,

                    #[serde(flatten)]
                    ty: ComponentType<Canonical>,

                    #[serde(default, skip_serializing_if = "Option::is_none")]
                    allow: Option<Allow>,
                },
            }

            let helper = Helper::deserialize(deserializer)?;
            match helper {
                Helper::Struct { id, ty, allow } => Ok(Component { id, ty, allow }),
            }
        }
    }

    impl<'de> Deserialize<'de> for Component<Thin> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            #[derive(Deserialize)]
            #[serde(untagged)]
            enum Helper {
                Struct {
                    id: ComponentId,

                    #[serde(flatten)]
                    ty: ComponentType<Thin>,

                    #[serde(default, skip_serializing_if = "Option::is_none")]
                    allow: Option<Allow>,
                },
            }

            let helper = Helper::deserialize(deserializer)?;
            match helper {
                Helper::Struct { id, ty, allow } => Ok(Component { id, ty, allow }),
            }
        }
    }

    impl<'de> Deserialize<'de> for Component<Encrypted> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            #[derive(Deserialize)]
            #[serde(untagged)]
            enum Helper {
                Struct {
                    id: ComponentId,

                    #[serde(flatten)]
                    ty: ComponentType<Encrypted>,

                    #[serde(default, skip_serializing_if = "Option::is_none")]
                    allow: Option<Allow>,
                },
            }

            let helper = Helper::deserialize(deserializer)?;
            match helper {
                Helper::Struct { id, ty, allow } => Ok(Component { id, ty, allow }),
            }
        }
    }

    impl<'de, C: ComponentState> Deserialize<'de> for Components<C>
    where
        Component<C>: Deserialize<'de>,
        // C::Id: Deserialize<'de>,
        // C::Media: Deserialize<'de>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let components = Vec::<Component<C>>::deserialize(deserializer)?;
            Ok(Components { inner: components })
        }
    }

    impl<C: ComponentState> Serialize for Components<C>
    where
        Component<C>: Serialize,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.inner.serialize(serializer)
        }
    }
}

#[cfg(feature = "utoipa")]
mod _u {
    use utoipa::{
        __dev::ComposeSchema,
        ToSchema,
        openapi::{RefOr, schema::Schema},
        schema,
    };

    use crate::v1::types::components::{Canonical, Create, Encrypted};

    use super::{Component, Components};

    impl ToSchema for Components<Create> {}
    impl ToSchema for Components<Canonical> {}
    impl ToSchema for Components<Encrypted> {}

    // HACK: this seems to be private, but i need to impl it anyways?
    impl ComposeSchema for Components<Create> {
        fn compose(_: Vec<RefOr<Schema>>) -> RefOr<Schema> {
            schema!(Vec<Component<Create>>).into()
        }
    }

    impl ComposeSchema for Components<Canonical> {
        fn compose(_: Vec<RefOr<Schema>>) -> RefOr<Schema> {
            schema!(Vec<Component<Canonical>>).into()
        }
    }

    impl ComposeSchema for Components<Encrypted> {
        fn compose(_: Vec<RefOr<Schema>>) -> RefOr<Schema> {
            schema!(Vec<Component<Encrypted>>).into()
        }
    }
}

mod flex {
    pub trait Seal {}
}

/// needs to be created
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Create {}

/// has been created
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Canonical {}

/// has been created, only has media ids instead of full media
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Thin {}

/// used for encrypted messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Encrypted {}

pub trait ComponentStateMedia: flex::Seal {
    fn media_id(&self) -> Option<MediaId>;
}

#[cfg(not(feature = "utoipa"))]
pub trait ComponentState: flex::Seal {
    type Id: Debug + Clone + Copy + PartialEq + Eq + Serialize + for<'de> Deserialize<'de>;

    type Media: Debug
        + Clone
        + PartialEq
        + ComponentStateMedia
        + Serialize
        + for<'de> Deserialize<'de>
        + Sync
        + Send; // Added for safety in async contexts
}

#[cfg(feature = "utoipa")]
pub trait ComponentState:
    flex::Seal + ToSchema + utoipa::PartialSchema + utoipa::__dev::ComposeSchema
{
    type Id: Debug
        + Clone
        + Copy
        + PartialEq
        + Eq
        + ToSchema
        + Serialize
        + for<'de> Deserialize<'de>;

    type Media: Debug
        + Clone
        + PartialEq
        + ToSchema
        + ComponentStateMedia
        + Serialize
        + for<'de> Deserialize<'de>
        + Sync
        + Send; // Added for safety in async contexts
}

/// Iterator over component children
pub enum ComponentChildren<'a, C: ComponentState> {
    Leaf,
    Container {
        children: std::slice::Iter<'a, Component<C>>,
    },
    Details {
        summary: std::slice::Iter<'a, Component<C>>,
        details: std::slice::Iter<'a, Component<C>>,
    },
}

impl flex::Seal for Create {}
impl flex::Seal for Canonical {}
impl flex::Seal for Thin {}
impl flex::Seal for Encrypted {}

impl ComponentState for Create {
    type Id = Option<ComponentId>;
    type Media = MediaReference;
}

impl ComponentState for Canonical {
    type Id = ComponentId;
    type Media = Media;
}

impl ComponentState for Thin {
    type Id = ComponentId;
    type Media = MediaId;
}

impl ComponentState for Encrypted {
    type Id = ComponentId;
    type Media = EncryptedMedia;
}

impl flex::Seal for MediaReference {}
impl flex::Seal for Media {}
impl flex::Seal for MediaId {}
impl flex::Seal for EncryptedMedia {}

impl ComponentStateMedia for Media {
    fn media_id(&self) -> Option<MediaId> {
        Some(self.id)
    }
}

impl ComponentStateMedia for MediaReference {
    fn media_id(&self) -> Option<MediaId> {
        match self {
            MediaReference::Media { media_id } => Some(*media_id),
            _ => None,
        }
    }
}

impl ComponentStateMedia for MediaId {
    fn media_id(&self) -> Option<MediaId> {
        Some(*self)
    }
}

impl ComponentStateMedia for EncryptedMedia {
    fn media_id(&self) -> Option<MediaId> {
        Some(self.id)
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
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(
        tag = "type",
        bound(
            serialize = "Component<C>: Serialize",
            deserialize = "Component<C>: Deserialize<'de>"
        )
    )
)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ComponentType<C: ComponentState> {
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
        #[cfg_attr(feature = "utoipa", schema(no_recursion))]
        components: Vec<Component<C>>,
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

        #[cfg_attr(feature = "utoipa", schema(no_recursion))]
        summary: Vec<Component<C>>,

        #[cfg_attr(feature = "utoipa", schema(no_recursion))]
        details: Vec<Component<C>>,
    },

    /// a section without any margin/padding
    Section {
        color: Option<Color>,

        #[cfg_attr(feature = "utoipa", schema(no_recursion))]
        components: Vec<Component<C>>,
    },

    /// display media
    ///
    /// min 1 max 20 items
    Media {
        items: Vec<ComponentMedia<C::Media>>,
    },

    /// display a carousel of media
    ///
    /// min 1 max 20 items
    Gallery {
        items: Vec<ComponentMedia<C::Media>>,
    },
}

impl<C: ComponentState> Component<C> {
    // should i remove this and access .ty directly?
    pub(super) fn ty(&self) -> &ComponentType<C> {
        &self.ty
    }
}

impl<C: ComponentState> Component<C> {
    /// Find a component by its ID recursively.
    pub fn find_by_id(&self, id: C::Id) -> Option<&Component<C>> {
        if self.id == id {
            return Some(self);
        }
        self.children().find_map(|c| c.find_by_id(id))
    }

    /// get an iterator over this component's children.
    pub fn children(&self) -> ComponentChildren<'_, C> {
        match &self.ty {
            ComponentType::Container { components, .. }
            | ComponentType::Section { components, .. } => ComponentChildren::Container {
                children: components.iter(),
            },
            ComponentType::Details {
                summary, details, ..
            } => ComponentChildren::Details {
                summary: summary.iter(),
                details: details.iter(),
            },
            _ => ComponentChildren::Leaf,
        }
    }

    /// Visit all component IDs in the tree recursively.
    pub fn visit_ids<F>(&self, f: &mut F)
    where
        F: FnMut(C::Id),
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

    /// Extract all media from the component tree.
    pub fn media_ids(&self) -> Vec<MediaId> {
        let mut ids = vec![];
        self.collect_media(&mut ids);
        ids
    }

    /// Recursively collect all media from the component tree.
    fn collect_media(&self, ids: &mut Vec<MediaId>) {
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
                    if let Some(id) = item.media.media_id() {
                        ids.push(id);
                    }
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

impl<C: ComponentState> Components<C> {
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn children(&self) -> ComponentChildren<'_, C> {
        ComponentChildren::Container {
            children: self.inner.iter(),
        }
    }
}

impl<'a, C: ComponentState> Iterator for ComponentChildren<'a, C> {
    type Item = &'a Component<C>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ComponentChildren::Leaf => None,
            ComponentChildren::Container { children } => children.next(),
            ComponentChildren::Details { summary, details } => {
                summary.next().or_else(|| details.next())
            }
        }
    }
}

impl<C: ComponentState> Default for Components<C> {
    fn default() -> Self {
        Self { inner: Vec::new() }
    }
}

impl ComponentMedia<Media> {
    pub fn into_thin(self) -> ComponentMedia<MediaId> {
        ComponentMedia {
            media: self.media.id,
            description: self.description,
            spoiler: self.spoiler,
        }
    }
}

impl Component<Canonical> {
    pub fn into_thin(self) -> Component<Thin> {
        Component {
            id: self.id,
            ty: self.ty.into_thin(),
            allow: self.allow,
        }
    }
}

impl Components<Canonical> {
    pub fn into_thin(self) -> Components<Thin> {
        Components {
            inner: self.inner.into_iter().map(|c| c.into_thin()).collect(),
        }
    }
}

impl ComponentType<Thin> {
    /// Convert a thin component type back into a create component type.
    ///
    /// This is used for generating initial flume deltas where the client
    /// needs to receive components in Create format.
    pub fn into_create(self) -> ComponentType<Create> {
        match self {
            ComponentType::Button {
                label,
                style,
                custom_id,
            } => ComponentType::Button {
                label,
                style,
                custom_id,
            },
            ComponentType::LinkButton { label, url } => ComponentType::LinkButton { label, url },
            ComponentType::Text { content } => ComponentType::Text { content },
            ComponentType::Reference { reference_id } => ComponentType::Reference { reference_id },
            ComponentType::Container { components, color } => ComponentType::Container {
                components: components.into_iter().map(|c| c.into_create()).collect(),
                color,
            },
            ComponentType::Section { components, color } => ComponentType::Section {
                components: components.into_iter().map(|c| c.into_create()).collect(),
                color,
            },
            ComponentType::Details {
                open,
                color,
                summary,
                details,
            } => ComponentType::Details {
                open,
                color,
                summary: summary.into_iter().map(|c| c.into_create()).collect(),
                details: details.into_iter().map(|c| c.into_create()).collect(),
            },
            ComponentType::Media { items } => ComponentType::Media {
                items: items
                    .into_iter()
                    .map(|i| ComponentMedia {
                        media: MediaReference::Media { media_id: i.media },
                        description: i.description,
                        spoiler: i.spoiler,
                    })
                    .collect(),
            },
            ComponentType::Gallery { items } => ComponentType::Gallery {
                items: items
                    .into_iter()
                    .map(|i| ComponentMedia {
                        media: MediaReference::Media { media_id: i.media },
                        description: i.description,
                        spoiler: i.spoiler,
                    })
                    .collect(),
            },
        }
    }
}

impl Component<Thin> {
    pub fn into_create(self) -> ComponentCreate {
        ComponentCreate {
            id: Some(self.id),
            ty: self.ty.into_create(),
            allow: self.allow,
        }
    }
}

impl Components<Thin> {
    pub fn into_create(self) -> Components<Create> {
        Components {
            inner: self.inner.into_iter().map(|c| c.into_create()).collect(),
        }
    }
}
