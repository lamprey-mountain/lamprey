use lamprey_macros::record;
use url::Url;

use crate::{
    v1::types::metadata::Metadata,
    v2::types::{
        MediaId,
        components::{ComponentId, ComponentType, interactive::Allow},
        media::{Media as ApiMedia, MediaReference},
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
    pub media: Vec<ApiMedia>,

    /// application-specific metadata
    // NOTE: maybe rename to `variables`? or have both variables to use with templates later and generic metadata?
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

/// a single component in a tree
#[record]
pub struct Component {
    pub id: ComponentId,

    #[serde(flatten)]
    pub ty: ComponentType,

    /// who's allowed to interact with this component
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

// TODO: impl and use? do i need this struct?
//
// in the past, components formed a json tree. now they're flat. maybe its still
// nice to be able to create components using a tree?
//
// in the past, when creating components, a String could be used as a shorthand
// to create a Text component with no content. since every component now is
// required to have an id and the structure is flat, i don't think this is
// possible? but having shortcuts to save bandwidth would be nice.
#[record]
#[serde(transparent)]
pub struct Create {
    inner: Components,
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

    // pub fn description(mut self, description: impl Into<String>) -> Self {
    //     self.description = Some(description.into());
    //     self
    // }

    /// set whether this media should be a spoiler
    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.spoiler = spoiler;
        self
    }
}

impl Create {
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

// TODO: impl From components for Create

impl From<MediaReference> for ComponentMedia {
    fn from(media_ref: MediaReference) -> Self {
        Self {
            media_ref,
            description: None,
            spoiler: false,
        }
    }
}

impl From<MediaId> for ComponentMedia {
    fn from(media_id: MediaId) -> Self {
        Self::new_media(media_id)
    }
}

impl From<ApiMedia> for ComponentMedia {
    fn from(media: ApiMedia) -> Self {
        Self {
            media_ref: MediaReference::Media { media_id: media.id },
            description: media.alt,
            spoiler: false,
        }
    }
}

impl From<&ApiMedia> for ComponentMedia {
    fn from(media: &ApiMedia) -> Self {
        Self {
            media_ref: MediaReference::Media { media_id: media.id },
            description: media.alt.clone(),
            spoiler: false,
        }
    }
}
