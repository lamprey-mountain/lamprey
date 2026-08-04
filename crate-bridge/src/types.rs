use serde::{Deserialize, Serialize};
use url::Url;

use crate::prelude::*;

macro_rules! genid {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(::uuid::Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(::uuid::Uuid::now_v7())
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<::uuid::Uuid> for $name {
            fn from(other: ::uuid::Uuid) -> Self {
                Self(other)
            }
        }
    };
}

genid!(PortalId);
genid!(RealmId);
genid!(MessageId);
genid!(PendingLinkId);

/// Tracks a pending link request while waiting for confirmation
#[derive(Debug, Clone)]
pub struct PendingLink {
    pub id: PendingLinkId,
    pub discord_guild_id: discord::GuildId,
    pub discord_channel_id: discord::ChannelId,
    pub lamprey_channel_id: lamprey::ChannelId,
    pub webhook_url: Url,
    pub confirmation_message_id: Option<lamprey::MessageId>,
}

/// a known/supported platform
#[derive(
    Debug, Clone, Serialize, Deserialize, strum::Display, strum::EnumString, PartialEq, Eq,
)]
pub enum Platform {
    Lamprey,
    Discord,
}

// TODO: use or remove
/// a single logical channel. forwards messages across platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalData {
    pub id: PortalId,
    pub realm_id: Option<RealmId>,
    pub links: Vec<PortalLink>,
}

/// a platform the portal is connected to
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state")]
pub enum PortalLink {
    Pending(PortalRequest),
    Live(PortalLinkType),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalRequest {
    pub platform: Platform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "platform")]
pub enum PortalLinkType {
    Lamprey {
        channel_id: lamprey::ChannelId,
        room_id: lamprey::RoomId,
        last_id: lamprey::MessageId,
    },

    Discord {
        guild_id: discord::GuildId,
        parent_id: Option<discord::ChannelId>, // for threads
        channel_id: discord::ChannelId,
        webhook_url: Url,
        last_id: discord::MessageId,
    },
}

// TODO: consider redesigning these types?
pub use may_redesign::*;
pub mod may_redesign {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct User {
        pub source_platform: Platform,
        pub lamprey_id: lamprey::UserId,
        pub discord_id: discord::UserId,

        // used for syncing media
        pub discord_avatar_url: Option<String>,
        pub discord_banner_url: Option<String>,
    }

    /// metadata for a single logical message
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Message {
        pub portal_id: PortalId,
        pub source_platform: Platform,

        pub lamprey_message_id: Option<lamprey::MessageId>,
        pub discord_message_id: Option<discord::MessageId>,
        /// media/attachment ids to know what needs to be uploaded on edit vs what can be reused
        pub attachments: Vec<(lamprey::MediaId, discord::AttachmentId)>,
    }

    #[derive(Debug, Clone)]
    pub struct PortalChannel {
        pub name: String,
        pub description: Option<String>,
        pub kind: ChannelKind,
        pub parent_id: Option<PortalId>,
        pub position: Option<u64>,
    }

    #[derive(Debug, Clone)]
    pub enum ChannelKind {
        Text,
    }

    #[derive(Debug, Clone)]
    pub struct Attachment {
        pub filename: String,
        pub bytes: Vec<u8>,
    }

    #[derive(Debug, Clone)]
    pub struct LampreyInfo {
        pub cdn_url: Url,
    }

    #[derive(Debug, Clone)]
    pub enum MessageData {
        Lamprey {
            message: Box<lamprey::Message>,
            user: Box<lamprey::User>,
            room_member: Option<Box<lamprey::RoomMember>>,
            info: Box<LampreyInfo>,
        },

        Discord {
            message: Box<discord::Message>,
        },
    }

    impl MessageData {
        // TODO
    }
}
