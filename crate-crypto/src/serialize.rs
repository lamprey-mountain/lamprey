//! importing and exporting data

use lamprey::v1::types::e2ee::MlsEpoch;

use crate::{Encryption, EncryptionChannel};

/// serialized encryption state for a session
///
/// does not include per-channel states
pub struct SerializedEncryption {
    // TODO
}

/// serialized encryption state for a channel
pub struct SerializedChannel {
    // TODO
    // channel id
}

/// serialized epoch key for a channel
pub struct SerializedKey {
    epoch: MlsEpoch,
    // TODO
    // key data: ...
}

impl Encryption {
    /// import serialized encryption state
    pub fn import(state: SerializedEncryption) -> Encryption {
        todo!()
    }

    /// export serialized encryption state
    pub fn export(&self) -> SerializedEncryption {
        todo!()
    }

    /// import serialized encryption state for a channel
    pub fn import_channel(&mut self, state: SerializedChannel) {
        todo!()
    }
}

impl EncryptionChannel {
    pub fn export(&self) -> SerializedChannel {
        // self.group.export_something
        todo!()
    }

    pub fn import_key(&mut self, key: SerializedKey) {
        todo!()
    }

    pub fn export_key(&self, epoch: MlsEpoch) -> SerializedKey {
        // should i just export all keys in SerializedChannel?
        todo!()
    }
}

// impl CrossSigning {
//     pub fn import(_user_id: UserId, _session_id: SessionId, _data: &[u8]) -> Result<Self> {
//         todo!()
//     }

//     pub fn export(&self) -> Result<Vec<u8>> {
//         todo!()
//     }
// }
