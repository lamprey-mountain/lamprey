//! types used inside a harvest archive
//!
//! the archive is a single zip file:
//!
//! ```text
//! readme.txt
//! harvest.json                 the contents of [`Metadata`]
//! media.ndjson                 every referenced media object, by id
//! users.ndjson                 every user referenced by the archive
//! audit-log.ndjson             user harvests only
//! rooms/<room_id>/room.json    the contents of [`RoomFile`]
//! rooms/<room_id>/emojis.ndjson
//! rooms/<room_id>/members.ndjson
//! rooms/<room_id>/audit-log.ndjson
//! channels/<channel_id>/channel.json
//! channels/<channel_id>/messages.ndjson
//! channels/<channel_id>/versions.ndjson
//! channels/<channel_id>/reactions.ndjson
//! ```
//!
//! every `*.ndjson` file contains one json value per line. `media` objects
//! embedded in messages are replaced by their id, the full object is in
//! `media.ndjson`.
//!
//! user harvests only contain the messages the user has sent (and their
//! versions), plus, if requested, the reactions the user has sent in channels
//! that are part of the archive.

use lamprey_macros::record;

use super::HarvestType;
use crate::v1::types::{
    Channel, HarvestId, MessageId, Role, Room, RoomBan, User, UserId, application::Application,
    misc::Time, reaction::ReactionKeyParam, server::ServerInfo,
};

/// the version of the archive layout
///
/// bump this when files are added, removed, or change shape.
pub const ARCHIVE_VERSION: u32 = 1;

/// the version of the api types that are serialized inside the archive
// NOTE: this is implicitly v1 since that's the mod we're in?
pub const ARCHIVE_API_VERSION: u16 = 1;

/// the contents of `harvest.json`
#[record]
pub struct Metadata {
    pub archive_version: u32,
    pub api_version: u16,

    pub harvest: HarvestInfo,

    /// the exported user, if this is a user harvest
    pub user: Option<User>,

    /// the exported room, if this is a room harvest
    pub room: Option<Room>,

    /// applications owned by the exported user
    pub applications: Vec<Application>,

    pub server: ServerInfo,
}

/// the harvest this archive was created from
#[record]
pub struct HarvestInfo {
    pub id: HarvestId,
    pub requester_id: UserId,
    pub queued_at: Time,
    pub started_at: Time,
    pub completed_at: Time,

    #[serde(flatten)]
    pub ty: HarvestType,
}

/// the contents of `rooms/<room_id>/room.json`
#[record]
pub struct RoomFile {
    pub room: Room,
    pub roles: Vec<Role>,
    pub channels: Vec<Channel>,

    /// only present if the requester is allowed to see bans
    pub bans: Vec<RoomBan>,
}

/// a single reaction, as written to `channels/<channel_id>/reactions.ndjson`
#[record]
pub struct ReactionArchive {
    pub message_id: MessageId,
    pub key: ReactionKeyParam,
    pub user_id: UserId,
    pub created_at: Time,
}

// TODO: include invites, webhooks, calendar events, documents, etc..?
