use lamprey::v1::types::e2ee::media::{EncryptedMediaAlgorithm, EncryptedMediaParams};

use crate::prelude::*;

// NOTE: this can be done with the web crypto api
// maybe i don't need to include this for wasm?

/// Encrypt raw media bytes with AES-256-GCM.
pub fn encrypt_media(
    data: &[u8],
    alg: &EncryptedMediaAlgorithm,
) -> Result<(Vec<u8>, EncryptedMediaParams)> {
    match alg {
        EncryptedMediaAlgorithm::Aes256GCM => todo!(),
    }
}

/// Decrypt raw media bytes with AES-256-GCM.
pub fn decrypt_media(data: &[u8], params: &EncryptedMediaParams) -> Result<Vec<u8>> {
    match params {
        EncryptedMediaParams::Aes256GCM { .. } => todo!(),
    }
}
