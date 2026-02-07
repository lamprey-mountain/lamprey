use std::collections::HashMap;

use url::Url;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{misc::Time, SfuId};

/// public moderation capabilities for a server
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerModeration {
    pub automod_lists: Vec<ServerAutomodList>,
    pub media_scanners: Vec<ServerMediaScanner>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerAutomodList {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerMediaScanner {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerInfo {
    /// the rest/http api base url
    pub api_url: Url,

    /// the websocket sync url
    // NOTE: this will pretty much always be api_url + /api/v1/sync for now
    pub sync_url: Url,

    /// the html web ui base url
    pub html_url: Url,

    /// the cdn base url
    pub cdn_url: Url,
    pub features: ServerFeatures,
    pub version: ServerVersion,
}

/// features that this server supports
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerFeatures {
    /// if present, indicates that this server is letting new users register
    pub registration: Option<ServerRegistration>,

    /// what authentication this server supports
    pub auth: Option<ServerAuth>,

    /// media configuration for this server, if supported
    pub media: Option<ServerMedia>,

    /// voice configuration for this server, if supported
    pub voice: Option<ServerVoice>,

    /// web push configuration for this server, if supported
    pub web_push: Option<ServerWebPush>,
    // TODO: add automod, calendar, documents, federation(?), search
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerRegistration {
    /// whether new people can register at all
    pub enabled: bool,
    // TODO: granular registration:
    // /// whether guest accounts can be created on this server
    // // NOTE: this should be always enabled, use guest_permissions instead?
    // guests_enabled: bool,
    //
    // /// the permissions that guests have
    // // create rooms, start dms, use voice, use video, etc
    // // maybe allow masking permissions
    // guest_permissions: Vec<_>,
    //
    // /// whether a server invite is required to join this server (DISABLING NOT RECOMMENDED)
    // invite_required: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerMedia {
    pub max_file_size: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerAuth {
    pub supports_totp: bool,
    pub supports_webauthn: bool,
    pub oauth_providers: Vec<ServerAuthOauth>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerAuthOauth {
    /// friendly name
    pub name: String,

    /// api name
    pub id: String,
    // TODO: more fields?
    // pub icon: MediaId,
    // pub application_id: ApplicationId,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerVoice {
    // currently empty
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerWebPush {
    pub vapid_public_key: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerVersion {
    /// the implementation thats being used
    pub implementation: String,

    /// the version of the implementation
    pub version: String,

    /// extra metadata for this server
    pub extra: HashMap<String, String>,
}

// maybe remove this and have user/room-specific constraints
// also could remove other limits above (eg. media max_file_size)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerLimits {
    // TODO: move crate-backend/src/consts.rs here?
}

/// sfu metadata for admins
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerVoiceSfu {
    /// a (temporary?) unique identifier for this sfu
    pub id: SfuId,

    /// when this sfu connected to the server
    pub connected_at: Time,

    /// the hostname of this sfu
    pub hostname: String,

    /// the ip address of this sfu
    pub address: String,

    /// the zone of this sfu (aka region, datacenter, etc)
    pub zone: String,

    /// total available bandwidth in bits per second
    pub bandwidth_total: u64,

    /// bandwidth that is being used in bits per second
    pub bandwidth_used: u64,

    /// number of users who are connected
    pub stat_users: u64,

    /// number of tracks this sfu is selectively forwarding
    pub stat_tracks: u64,
    // TODO: add version
}
