use crate::v1::types::{ChannelId, e2ee::MlsEpoch};
use lamprey_macros::record;

// see https://spec.matrix.org/v1.19/client-server-api/#construction-and-sharing-of-the-key-bundle

#[record]
pub struct KeyBundle {
    // channel_id
    pub keys: Vec<Key>,
    // pub witheld: Vec<()>,
}

#[record]
pub struct Key {
    pub epoch: MlsEpoch,
    pub data: Vec<u8>,
}

#[record]
pub struct KeyshareRequest {
    /// the channel id of the messages you want
    pub channel_id: ChannelId,

    /// start sending keys from this epoch
    pub start: MlsEpoch,

    /// send up to this many epochs of data
    #[validate(range(min = 1, max = 128))]
    pub limit: u8,

    /// HPKE public key used to encrypt the response keyring data
    #[validate(length(min = 1, max = 1024))]
    pub hpke_pub_key: Vec<u8>,
}

/// historical encryption keys for old messages
#[record]
pub struct KeyshareRespond {
    /// keyring data encrypted with the current
    // TODO: what does it look like decrypted? json probably?
    #[validate(length(min = 1))]
    pub encrypted_keyring_data: Vec<u8>,

    /// the channel (mls group) these keys are for
    pub channel_id: ChannelId,
}

#[record]
pub struct SessionKeyUploadRequest {
    // TODO: fix this type
    pub keys: Vec<u8>,
}

pub struct KeyringData {
    // TODO: think of what goes here
}
