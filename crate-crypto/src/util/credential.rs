use lamprey::v2::types::{SessionId, UserId};
use openmls::credentials::{Credential, CredentialType};
use serde::{Deserialize, Serialize};

/// number indicating that a credential is a lamprey credential
// this number was randomly generated, maybe i should choose one with some specific meaning?
// note that private use is in 0xF000 - 0xFFFF
pub const LAMPREY_CREDENTIAL_TYPE: CredentialType = CredentialType::Other(0xF18F);

// TODO: use this instead of BasicCredential
// TODO: flesh out this type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LampreyCredential {
    pub user_id: UserId,
    pub session_id: SessionId,
    // Signature over (user_id + session_id) using the Self-Signing Key
    // pub device_signature: lamprey_common::v1::types::e2ee::DeviceSignature,

    // /// signature over (user_id || session_id || mls_signature_key)
    // /// made with this device's cross-signing SSK.
    // /// this is what lets a peer verify "this MLS credential really
    // /// belongs to a cross-signed device", since MLS itself won't check it.
    // pub ssk_signature: Vec<u8>,
}

/// bytes that ssk_signature actually signs over
fn signed_payload(user_id: &UserId, session_id: &SessionId, mls_pubkey: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(user_id.as_bytes());
    buf.extend_from_slice(session_id.as_bytes());
    buf.extend_from_slice(mls_pubkey);
    buf
}

impl LampreyCredential {
    // pub fn new_signed(
    //     user_id: UserId,
    //     session_id: SessionId,
    //     mls_pubkey: &[u8],
    //     ssk: &ed25519_dalek::SigningKey,
    // ) -> Self {
    //     let payload = signed_payload(&user_id, &session_id, mls_pubkey);
    //     let sig = ssk.sign(&payload).to_bytes().to_vec();
    //     Self {
    //         user_id,
    //         session_id,
    //         ssk_signature: sig,
    //     }
    // }

    // /// verify this credential was actually signed by the claimed user's SSK
    // pub fn verify(&self, mls_pubkey: &[u8], ssk_verifying_key: &VerifyingKey) -> Result<(), Error> {
    //     let payload = Self::signed_payload(&self.user_id, &self.session_id, mls_pubkey);
    //     let sig =
    //         Signature::from_slice(&self.ssk_signature).map_err(|_| Error::SignatureInvalid)?;
    //     ssk_verifying_key
    //         .verify(&payload, &sig)
    //         .map_err(|_| Error::SignatureInvalid)
    // }
}

impl From<LampreyCredential> for Credential {
    fn from(value: LampreyCredential) -> Self {
        let bytes = postcard::to_stdvec(&value).expect("credential serialization is infallible");
        Credential::new(LAMPREY_CREDENTIAL_TYPE, bytes)
    }
}

/// an error that may occur when attempting to parse a LampreyCredential from a Credential
// TODO: impl conversion to error
#[derive(Debug, thiserror::Error)]
pub enum LampreyCredentialParseError {
    /// wrong credential type
    #[error("wrong credential type")]
    WrongType,

    /// deserialization error
    #[error("deserialization error")]
    DeserializationError,
}

impl TryFrom<Credential> for LampreyCredential {
    type Error = LampreyCredentialParseError;

    fn try_from(credential: Credential) -> Result<Self, Self::Error> {
        if credential.credential_type() != LAMPREY_CREDENTIAL_TYPE {
            return Err(LampreyCredentialParseError::WrongType);
        }
        postcard::from_bytes(credential.serialized_content())
            .map_err(|_| LampreyCredentialParseError::DeserializationError)
    }
}
