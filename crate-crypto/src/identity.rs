use ed25519_dalek::ed25519::signature::rand_core::OsRng;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use lamprey::v1::types::{SessionId, UserId};
use std::collections::HashMap;

use crate::prelude::*;

// /// Device identity management for cross-signing.
// ///
// /// Holds the master key, self-signing key, and user-signing key.
// /// Used to verify that devices belong to a user and to sign
// /// key packages for new device addition.
// pub struct CrossSigning {
//     user_id: UserId,
//     session_id: SessionId,
//     master_key: SigningKey,
//     self_signing_key: SigningKey,
//     user_signing_key: SigningKey,

//     signatures: HashMap<Vec<u8>, Vec<()>>,
// }

/// verification and cross signing state
pub struct CrossSigning {
    user_id: UserId,
    master_key: SigningKey,
    self_signing_key: SigningKey,
    user_signing_key: SigningKey,
    // keys: (),
    // signatures: (),
}

impl CrossSigning {
    /// create a new cross-signing identity
    pub fn generate(user_id: UserId) -> Self {
        // TODO: update ed25519 crate
        // let mut csprng: StdRng = rand::make_rng();
        let mut csprng = OsRng;
        let mk = SigningKey::generate(&mut csprng);
        let ssk = SigningKey::generate(&mut csprng);
        let usk = SigningKey::generate(&mut csprng);

        Self {
            user_id,
            master_key: mk,
            self_signing_key: ssk,
            user_signing_key: usk,
        }
    }

    // /// calculate a cross-signing bundle to upload to the server
    // pub fn sign_bundle(&self) -> CrossSigningBundle {
    //     let mk_pub = self.master_key.verifying_key().to_bytes().to_vec();
    //     let ssk_pub = self.self_signing_key.verifying_key().to_bytes().to_vec();
    //     let usk_pub = self.user_signing_key.verifying_key().to_bytes().to_vec();

    //     let mut ssk_sigs = HashMap::new();
    //     ssk_sigs.insert(
    //         "master".to_string(),
    //         self.master_key.sign(&ssk_pub).to_bytes().to_vec(),
    //     );

    //     let mut usk_sigs = HashMap::new();
    //     usk_sigs.insert(
    //         "master".to_string(),
    //         self.master_key.sign(&usk_pub).to_bytes().to_vec(),
    //     );

    //     CrossSigningBundle {
    //         master_key: Some(CrossSigningKey {
    //             key: mk_pub,
    //             signatures: HashMap::new(),
    //             usage: Usage::Master,
    //             session_id: self.session_id,
    //             user_id: self.user_id,
    //         }),
    //         self_signing_key: Some(CrossSigningKey {
    //             key: ssk_pub,
    //             signatures: ssk_sigs,
    //             usage: Usage::SelfSigning,
    //             session_id: self.session_id,
    //             user_id: self.user_id,
    //         }),
    //         user_signing_key: Some(CrossSigningKey {
    //             key: usk_pub,
    //             signatures: usk_sigs,
    //             usage: Usage::UserSigning,
    //             session_id: self.session_id,
    //             user_id: self.user_id,
    //         }),
    //     }
    // }

    // /// attempt to verify another session
    // pub fn verify_session(&self, id: SessionId) -> bool {
    //     // Alice’s device is using a master signing key that has signed her user-signing key,
    //     // Alice’s user-signing key has signed Bob’s master signing key,
    //     // Bob’s master signing key has signed Bob’s self-signing key, and
    //     // Bob’s self-signing key has signed Bob’s device key.
    //     todo!()
    // }

    // /// Sign a key package for device verification.
    // ///
    // /// Returns a `CrossSigningSignature` that can be uploaded
    // /// to prove this session owns the key package.
    // pub fn sign_key_package(&self, key_package_data: &[u8]) -> CrossSigningSignature {
    //     let sig = self.self_signing_key.sign(key_package_data).to_bytes().to_vec();
    //     CrossSigningSignature {
    //         user_id: Some(self.user_id),
    //         session_id: Some(self.session_id),
    //         key_id: "self_signing".to_string(),
    //         signature: sig,
    //     }
    // }

    // /// Verify another user's cross-signing bundle.
    // ///
    // /// Checks that the signatures on the bundle are valid and
    // /// that the keys form a valid chain of trust.
    // pub fn verify_bundle(&self, bundle: &CrossSigningBundle) -> Result<(), Error> {
    //     let mk = bundle.master_key.as_ref().ok_or(Error::MlsError("Missing master key".into()))?;
    //     let ssk = bundle.self_signing_key.as_ref().ok_or(Error::MlsError("Missing SSK".into()))?;
    //     let usk = bundle.user_signing_key.as_ref().ok_or(Error::MlsError("Missing USK".into()))?;

    //     let mk_verify = VerifyingKey::from_bytes(mk.key.as_slice().try_into().map_err(|_| Error::MlsError("Invalid MK".into()))?)
    //         .map_err(|_| Error::MlsError("Invalid MK".into()))?;

    //     let ssk_sig = ssk.signatures.get("master").ok_or(Error::MlsError("Missing SSK signature".into()))?;
    //     let ssk_sig = Signature::from_slice(ssk_sig).map_err(|_| Error::MlsError("Invalid SSK signature".into()))?;
    //     mk_verify.verify(&ssk.key, &ssk_sig).map_err(|_| Error::MlsError("SSK signature verification failed".into()))?;

    //     let usk_sig = usk.signatures.get("master").ok_or(Error::MlsError("Missing USK signature".into()))?;
    //     let usk_sig = Signature::from_slice(usk_sig).map_err(|_| Error::MlsError("Invalid USK signature".into()))?;
    //     mk_verify.verify(&usk.key, &usk_sig).map_err(|_| Error::MlsError("USK signature verification failed".into()))?;

    //     Ok(())
    // }
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
