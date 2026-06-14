use serde::{Deserialize, Serialize};

use common::v1::types::federation::ServerKeyAlgorithm;
use common::v1::types::util::Time;

/// a server's signing key for internal use (includes private key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerKeyInternal {
    /// the key algorithm
    pub alg: ServerKeyAlgorithm,

    /// public key (base64 url safe unpadded)
    pub pubkey: String,

    /// private key (base64 url safe unpadded)
    pub privkey: String,

    /// when this key expires
    pub expires_at: Time,
}

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

    /// federation signing keys
    #[serde(default)]
    pub federation_keys: Vec<ServerKeyInternal>,
}
