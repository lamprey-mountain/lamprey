use crate::prelude::*;

// TODO: use or remove
#[derive(Debug, Clone)]
pub struct Portal {
    // TODO: recv events
    data: PortalData,
    // links: Vec<...>,
    // pending: Vec<...>,
}

/// a single logical channel. forwards messages across platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalData {
    pub id: PortalId,
    pub realm_id: Option<RealmId>,
    // pub links: Vec<PortalLink>,
}

// /// a platform the portal is connected to
// #[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(tag = "state")]
// pub enum PortalLink {
//     Pending(PortalRequest),
//     Live(PortalLinkType),
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct PortalRequest {
//     pub platform: Platform,
// }

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalLamprey {
    pub channel_id: lamprey::ChannelId,
    pub room_id: lamprey::RoomId,
    pub last_id: lamprey::MessageId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalDiscord {
    pub guild_id: discord::GuildId,
    pub parent_id: Option<discord::ChannelId>, // for threads
    pub channel_id: discord::ChannelId,
    pub webhook_url: Url,
    pub last_id: discord::MessageId,
}
