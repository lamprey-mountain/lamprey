use url::Url;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

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
}
