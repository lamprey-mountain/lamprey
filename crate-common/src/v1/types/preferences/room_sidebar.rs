use lamprey_macros::record;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use uuid::Uuid;

use crate::v1::types::{ChannelId, RoomId};

// probably won't do these?
// TODO: server side validate that rooms/channels exist?
// TODO: server side automatically edit config as rooms/channels are updated/deleted?
// TODO: server side automatically edit config when room joined?

// probably will do this?
// TODO: server side enforce types, validatation

/// room navigation sidebar configuration
// TODO: make record macro compatible with this
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
// TODO: validate length
pub struct Sidebar(pub Vec<Toplevel>);

#[record]
#[serde(tag = "type")]
pub enum Item {
    Room { room_id: RoomId },
    View(View),
}

#[record]
#[serde(tag = "type")]
pub enum Toplevel {
    Folder(Folder),

    // NOTE: maybe inline Item?
    #[serde(untagged)]
    Item(Item),
}

/// an ordered collection of rooms or views
#[record]
pub struct Folder {
    pub id: Uuid,
    pub name: String,
    // TODO: validate length
    pub items: Vec<Item>,
}

/// a fake/virtual room with a custom channel nav
#[record]
pub struct View {
    pub id: Uuid,
    pub name: String,
    // TODO: validate lengths
    pub uncategorized_channels: Vec<ViewChannel>,
    pub categories: Vec<ViewCategory>,
}

#[record]
#[serde(untagged)]
pub enum ViewCategory {
    Inline(ViewChannel),

    Custom {
        name: String,
        // TODO: validate length
        channels: Vec<ViewChannel>,
    },
}

#[record]
pub struct ViewChannel {
    pub id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
}

impl Sidebar {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
