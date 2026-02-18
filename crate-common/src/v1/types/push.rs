#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PushCreate {
    pub endpoint: String,
    pub keys: PushCreateKeys,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PushCreateKeys {
    pub p256dh: String,
    pub auth: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PushInfo {
    /// the endpoint that web push payloads are sent to
    pub endpoint: String,

    /// the server's vapid key
    pub server_key: String,
}

#[cfg(any())]
mod next {
    // TODO: implement other push notification providers?
    enum PushProvider {
        /// web push
        // server: vapid key
        // client: endpoint, keys
        Web,

        /// apple push notification service (apns)
        // server: app_id
        // client: token
        Apple,

        /// google cloud messaging (gcm)
        // server: app_id
        // client: token
        Google,
    }
}
