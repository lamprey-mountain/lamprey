use std::collections::HashMap;

use lamprey_macros::record;

use crate::v1::types::{SessionId, UserId, misc::binary::Binary};

/// retrieve cross signing keys
#[record]
pub struct CrossSigningQueryRequest {
    pub users: Vec<UserId>,
    // /// get the keys for these users' sessions
    // ///
    // /// if an empty array is passed, get all sessions.
    // pub keys: HashMap<UserId, Vec<SessionId>>,
}

#[record]
pub struct CrossSigningQuery {
    pub bundles: HashMap<UserId, CrossSigningBundle>,
    // pub identities: HashMap<UserId, CrossSigningBundle>,
    // pub signatures: HashMap<UserId, CrossSigningSignature>,
    // pub devices: Vec<MlsKeyPackage>,
}

// #[record]
// pub struct KeysClaimRequest {
//     // pub keys: Vec<MlsKeyPackage>,
// }

// #[record]
// pub struct KeysUploadRequest {
//     // pub keys: Vec<MlsKeyPackage>,

//     // #[body]
//     // pub package: Bytes,
// }

// /// A signature created by a device key
// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// // TODO: verify max length is correct
// pub struct DeviceSignature(pub Binary<256>);

/// cross signing key
///
/// used to verify that your devices are trustworthy
#[record]
pub struct CrossSigningKey {
    pub key: Vec<u8>,

    // TODO: redo this type
    // pub signatures: HashMap<String, Vec<u8>>,
    pub usage: CrossSigningUsage,
    // remove these?
    // pub session_id: SessionId,
    // pub user_id: UserId,
}

#[record]
pub struct CrossSigningSignatures {
    pub signatures: Vec<CrossSigningSignature>,
}

#[record]
pub struct CrossSigningSignature {
    pub signature: Vec<u8>,
    // pub user_id: Option<UserId>,
    // pub session_id: Option<SessionId>,
    // pub key_id: String, // what format is this?
    // // TODO: verify length is correct
    // pub signature: Binary<32>,
}

// TODO: use these?
// #[record]
// pub enum CrossSigningAlgorithm {
//     #[serde(rename = "ED25519")]
//     Ed25519,
// }

// #[record]
// #[serde(tag = "alg")]
// pub enum CrossSigningKeyData {
//     #[serde(rename = "ED25519")]
//     Ed25519 { key: Binary<32> },
// }

// #[record]
// #[serde(tag = "alg")]
// pub enum CrossSigningSignatureData {
//     #[serde(rename = "ED25519")]
//     Ed25519 { signature: Binary<64> },
// }

/// what this cross signing key can be used for
#[record]
pub enum CrossSigningUsage {
    /// the root of trust
    Master,

    /// this key is used to sign your devices
    SelfSigning,

    /// this key is used to sign other users
    UserSigning,
}

/// a set of cross signing keys for a user
#[record]
pub struct CrossSigningBundle {
    /// master key: root of trust, signs the ssk and usk
    pub master: CrossSigningKey,

    /// self signing key: signs the user's own devices
    pub ssk: CrossSigningKey,

    /// user signing key: signs other user's master keys upon verification
    pub usk: CrossSigningKey,
}

// TODO: backup cross signing private keys. some methods:
// - have sessions gossip the keys to each other
// - encrypt and backup on the server
