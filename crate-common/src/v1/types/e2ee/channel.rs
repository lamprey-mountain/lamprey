use lamprey_macros::record;

// TODO: add field
// #[record]
// pub struct Channel {
//     pub encryption: Option<ChannelEncryption>,
// }

/// e2ee configuration for a channel
#[record]
pub struct ChannelEncryption {
    pub algorithm: ChannelEncryptionAlgorithm,
    pub rotation_period_ms: u64,
    pub rotation_period_msgs: u64,
    // /// keyring data encrypted with the current
    // // TODO: what does it look like decrypted? json probably?
    // #[cfg_attr(feature = "validator", validate(length(min = 1)))]
    // pub encrypted_keyring_data: Vec<u8>,
    // TODO: voice channel configuration?
}

/// algorithm/ciphersuite used to encrypt channel messages
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum ChannelEncryptionAlgorithm {
    /// messaging layer security version 1
    ///
    /// DH KEM x25519 | AES-GCM 128 | SHA2-256 | Ed25519
    #[default]
    MlsV1,
}

impl Default for ChannelEncryption {
    fn default() -> Self {
        Self {
            algorithm: Default::default(),

            // rotate keys after one week or 100 messages
            rotation_period_ms: 604800000,
            rotation_period_msgs: 100,
        }
    }
}
