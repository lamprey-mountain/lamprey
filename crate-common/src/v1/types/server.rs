use std::collections::HashMap;

use lamprey_macros::record;
use url::Url;

use crate::v1::types::{SfuId, misc::Time};

/// public moderation capabilities for a server
#[record]
pub struct ServerModeration {
    pub automod_lists: Vec<ServerAutomodList>,
    pub media_scanners: Vec<ServerMediaScanner>,
}

#[record]
pub struct ServerAutomodList {
    pub name: String,
    pub description: String,
}

#[record]
pub struct ServerMediaScanner {
    pub name: String,
    pub description: String,
}

#[record]
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
#[record]
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

#[record]
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

#[record]
pub struct ServerMedia {
    pub max_file_size: u64,
}

#[record]
pub struct ServerAuth {
    pub supports_totp: bool,
    pub supports_webauthn: bool,
    pub oauth_providers: Vec<ServerAuthOauth>,
}

#[record]
pub struct ServerAuthOauth {
    /// friendly name
    pub name: String,

    /// api name
    pub id: String,
    // TODO: more fields?
    // pub icon: MediaId,
    // pub application_id: ApplicationId,
}

#[record]
pub struct ServerVoice {
    // currently empty
}

#[record]
pub struct ServerWebPush {
    pub vapid_public_key: String,
}

// NOTE: maybe i should include supported api versions for federation (and expose supported versions on a top level endpoint)
#[record]
pub struct ServerVersion {
    /// the implementation thats being used
    pub implementation: String,

    /// the semantic version of the implementation
    // NOTE: how do i handle invalid semver?
    pub version: String,

    /// extra metadata for this server
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub extra: HashMap<String, String>,
}

/// voice (calls) service status for admins
#[record]
pub struct ServerVoiceHealth {
    /// sfu stats
    pub sfus: Vec<ServerVoiceHealthSfu>,
    // TODO: calls, voice states, issues?
}

/// sfu metadata for admins
#[record]
pub struct ServerVoiceHealthSfu {
    /// an (ephemeral?) unique identifier for this sfu
    pub id: SfuId,

    /// when this sfu connected to the server
    pub connected_at: Time,

    /// bandwidth that is being used in bits per second
    pub bandwidth_usage: u64,

    /// total available bandwidth in bits per second
    pub bandwidth_max: u64,

    /// number of total rtc connections
    pub count_peer: u64,

    /// number of users who are connected
    pub count_users: u64,

    /// number of tracks this sfu is selectively forwarding
    pub count_tracks: u64,
    // TODO: add version

    // /// the hostname of this sfu
    // pub hostname: String,

    // /// the ip address of this sfu
    // pub address: String,

    // /// the zone of this sfu (aka region, datacenter, etc)
    // pub zone: String,
}
