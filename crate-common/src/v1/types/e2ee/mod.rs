use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{misc::binary::Binary, ChannelId, SessionId, UserId};

pub mod media;

/// a mls epoch number, incremented each time the group membership changes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MlsEpoch(pub u64);

/// A signature created by a device key
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
// TODO: verify length is correct
pub struct DeviceSignature(pub Binary<256>);

/// a mls key package, uploaded by sessions for use in welcomes
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MlsKeyPackage {
    pub user_id: UserId,
    pub session_id: SessionId,

    /// opaque mls key package data
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 65535)))]
    pub data: Vec<u8>,
}

/// a welcome message used to add a new member to an mls group
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MlsWelcome {
    /// the opaque welcome message data (MLS Welcome message).
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4194304)))]
    pub data: Vec<u8>,

    /// the session that sent this welcome
    pub sender_id: SessionId,

    /// the channel (mls group) to join
    pub channel_id: ChannelId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MlsWelcomeCreate {
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4194304)))]
    pub data: Vec<u8>,
}

/// an mls commit message, representing group state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MlsCommit {
    /// opaque data
    ///
    /// is a commit or proposal for member add, remove, update
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4194304)))]
    pub data: Vec<u8>,

    /// the session that authored this message
    pub sender_id: SessionId,

    /// the channel (mls group) this takes place in
    pub channel_id: ChannelId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MlsCommitCreate {
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4194304)))]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct KeyshareRequest {
    /// the channel id of the messages you want
    pub channel_id: ChannelId,

    /// start sending keys from this epoch
    pub start: MlsEpoch,

    /// send up to this many epochs of data
    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 128)))]
    pub limit: u8,

    /// HPKE public key used to encrypt the response keyring data
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
    pub hpke_pub_key: Vec<u8>,
}

/// historical encryption keys for old messages
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct KeyshareResponse {
    /// keyring data encrypted with the current
    // TODO: what does it look like decrypted? json probably?
    #[cfg_attr(feature = "validator", validate(length(min = 1)))]
    pub encrypted_keyring_data: Vec<u8>,

    /// the channel (mls group) these keys are for
    pub channel_id: ChannelId,
}

pub struct KeyringData {
    // TODO: think of what goes here
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EncryptionConfig {
    // /// keyring data encrypted with the current
    // // TODO: what does it look like decrypted? json probably?
    // #[cfg_attr(feature = "validator", validate(length(min = 1)))]
    // pub encrypted_keyring_data: Vec<u8>,

    // /// the channel (mls group) these keys are for
    // pub channel_id: ChannelId,
}

pub enum EncryptionSystem {
    /// the default encryption system
    ///
    /// uses messaging layer security to exchange keys and group membership and aes-gcm-256 to encrypt messages
    MlsAes {
        // any config here?
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct KeysQuery {
    /// get the keys for these users' sessions. if an empty array is passed, get all sessions.
    pub keys: HashMap<UserId, Vec<SessionId>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct KeysQueryResponse {
    pub identities: HashMap<UserId, CrossSigningBundle>,
    pub signatures: HashMap<UserId, CrossSigningSignature>,
    pub devices: Vec<MlsKeyPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct KeysClaim {
    pub keys: Vec<MlsKeyPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CrossSigningBundle {
    /// root of trust, signs the ssk and usk
    pub master_key: Option<CrossSigningKey>,

    /// signs the user's own devices
    pub self_signing_key: Option<CrossSigningKey>,

    /// signs other user's master keys upon verification
    pub user_signing_key: Option<CrossSigningKey>,
}

/// a key for verifying your devices are trustworthy
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CrossSigningKey {
    pub key: Vec<u8>,
    pub signatures: HashMap<String, Vec<u8>>,
    pub usage: Usage,

    pub session_id: SessionId,
    pub user_id: UserId,
}

/// what this cross signing key can be used for
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Usage {
    /// the root of trust
    Master,

    /// this key is used to sign your devices
    SelfSigning,

    /// this key is used to sign other users
    UserSigning,
}

/// a signature
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CrossSigningSignature {
    pub user_id: Option<UserId>,
    pub session_id: Option<SessionId>,
    pub key_id: String, // what format is this?
    // TODO: verify length is correct
    pub signature: Binary<32>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum E2EEMessage {
    /// someone wants to join a channel
    ///
    /// - a commit should be generated to allow this person to join
    /// - sent to one person in the group
    /// - prioritizes sending to a session from the same user
    MlsKnock {
        channel_id: ChannelId,

        /// the key package of the person who wants to join
        // the server grabs a random mls key package and sends it here
        key_package: MlsKeyPackage,
    },

    /// a new device has been added to one or more channels
    ///
    /// sent to the one session that is joining
    MlsWelcome {
        recipient_id: SessionId,

        #[cfg_attr(feature = "serde", serde(flatten))]
        welcome: MlsWelcome,
    },

    // /// mls group membership(s) changed, update your local state
    // ///
    // /// sent to everyone in the group(s)
    // MlsCommit {
    //     #[cfg_attr(feature = "serde", serde(flatten))]
    //     commit: MlsCommit,
    // },
    /// a mls protocol message (commit, proposal, or application data)
    MlsMessage {
        /// the session that authored this message
        sender_id: SessionId,

        /// the channel (mls group) this takes place in
        channel_id: ChannelId,

        /// the opaque mls ProtocolMessage bytes
        // TODO: find an appropriate size limit for this
        data: Binary<4194304>,
    },

    /// how many keys a session has uploaded
    ///
    /// consider uploading more key data if count is low
    MlsKeyCount {
        user_id: UserId,
        session_id: SessionId,
        count: u32,
    },

    /// someone wants access to message history
    ///
    /// sent to one person in the group
    KeyshareRequest {
        sharer_id: SessionId,

        nonce: String,

        #[cfg_attr(feature = "serde", serde(flatten))]
        request: KeyshareRequest,
    },

    /// here are your encryption keys
    ///
    /// sent from sharer -> server and server -> requester
    KeyshareResponse {
        /// who to send to, only usable and set by server
        recipient_id: Option<SessionId>,

        /// nonce to know which request this is associated with
        ///
        /// - sharer should set to nonce that sevrer set on E2EEKeyshareRequest
        /// - server should set to requester's nonce
        nonce: String,

        response: KeyshareResponse,
    },

    /// cross signing identity updated
    IdentityUpdated {
        user_id: UserId,
        bundle: CrossSigningBundle,
    },

    /// cross signing signature added
    SignatureAdded {
        user_id: UserId,

        #[cfg_attr(feature = "serde", serde(flatten))]
        signature: CrossSigningSignature,
    },
}

/*
cross signing

init

1. generate mk, ssk, usk
2. sign ssk and usk with mk
3. upload keys to api

verifying

1. take key package of new device
2. sign session id
3. upload signature

real time update with E2EESignatureAdded
*/
