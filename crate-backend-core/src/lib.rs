pub mod data;
pub mod error;
pub mod state;

// TEMP: clean up these types
// - some of this should be moved to data
// - some of this should be moved to common
// - some of this should be stabilized
pub mod types;

pub use error::{Error, Result};
use serde::{Deserialize, Serialize};
pub use state::{ServerState, ServerStateInner};

/// internal config that is saved in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInternal {
    /// web push api vapid public key
    pub vapid_private_key: String,

    /// web push api vapid private key
    pub vapid_public_key: String,

    /// openid connect key
    pub oidc_jwk_key: String,

    /// a token that can be used to do administrative operations on this server
    ///
    /// - DO NOT LEAK THIS TOKEN!
    /// - if this is None, there is no valid token
    /// - this gets rotated every 5 minutes
    /// - cli tools will fetch this token from the db, then do admin tasks through the http api
    pub admin_token: Option<String>,
}
