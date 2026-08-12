use std::collections::HashMap;

use lamprey::{
    v1::types::e2ee::channel::{ChannelEncryption, ChannelEncryptionAlgorithm},
    v2::types::{ChannelId, sync::E2EEDispatch},
};
use openmls::{
    credentials::{BasicCredential, CredentialWithKey},
    group::{GroupId, MlsGroup},
    key_packages::KeyPackage,
    prelude::{Ciphersuite, OpenMlsProvider},
};
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;

use crate::{
    group::EncryptionChannel, identity::CrossSigning, manager::verification::PeerVerification,
    prelude::*, util::credential::LampreyCredential,
};

pub mod verification;

/// main entry point
// #[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct Encryption {
    shared: Ref<EncryptionShared>,
    channels: HashMap<ChannelId, EncryptionChannel>,
    identity: CrossSigning,
    verification: PeerVerification,
}

/// shared encryption state
pub struct EncryptionShared {
    /// cryptography provider
    pub(crate) provider: OpenMlsRustCrypto,

    pub(crate) credential: LampreyCredential,
    // storage: Box<dyn AsyncKVStore>,
    // signer: openmls_basic_credential::SignatureKeyPair,
}

// KeysUpload(Vec<lamprey_common::v1::types::e2ee::MlsKeyPackage>),
// KeysQuery, // TODO: define query args
// KeysClaim, // TODO: define claim args
// UploadCrossSigningBundle(CrossSigningBundle),
// UploadCrossSigningSignature(CrossSigningSignature),
// GroupCommit(ChannelId, MlsCommitCreate),
// GroupWelcome(ChannelId, MlsWelcomeCreate),
// /// Emitted after handling an MlsKnock. Server applies the commit and sends the welcome.
// AddMember(ChannelId, MlsCommitCreate, MlsWelcomeCreate),
// KeyshareRequest(lamprey_common::v1::types::e2ee::KeyshareRequest),
/// an action that the caller to do
pub enum Action {
    /// upload all these keys to the api server
    UploadKeys(Vec<()>),
}

pub type Actions = Vec<Action>;

// #[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Encryption {
    /// create a new encryption manager from scratch
    // #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new() -> Encryption {
        let provider = OpenMlsRustCrypto::default();
        let cred = BasicCredential::new(vec![1, 2, 3, 4]);

        let ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;
        // let cred = LampreyCredential {
        //     user_id: todo!(),
        //     session_id: todo!(),
        // };
        let keys = SignatureKeyPair::new(ciphersuite.signature_algorithm()).unwrap();
        // keys.public()
        keys.store(provider.storage()).unwrap();
        let cred = CredentialWithKey {
            credential: cred.into(),
            signature_key: keys.public().into(),
        };

        let pkg = KeyPackage::builder()
            .build(ciphersuite, &provider, &keys, cred.clone())
            .unwrap();

        // generate a dozen or so key packages and upload them (Action::KeysUpload)

        todo!()
    }

    pub fn create_channel(&mut self, channel_id: ChannelId, config: &ChannelEncryption) -> Actions {
        let ciphersuite = match &config.algorithm {
            ChannelEncryptionAlgorithm::MlsV1 => {
                Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519
            }
        };

        let group_id = GroupId::from_slice(channel_id.as_bytes());

        let keys = SignatureKeyPair::new(ciphersuite.signature_algorithm()).unwrap();
        // keys.public()
        keys.store(self.shared.provider.storage()).unwrap();
        let creds = CredentialWithKey {
            credential: self.shared.credential.clone().into(),
            signature_key: keys.public().into(),
        };

        // MlsGroup::load(storage, group_id)

        let group = MlsGroup::builder()
            .with_group_id(group_id)
            // .max_past_epochs(max_past_epochs)
            // .ciphersuite(ciphersuite)
            .build(&self.shared.provider, &keys, creds)
            .unwrap();

        // group.commit_builder().add_proposal(proposal);
        // group.members().next().unwrap()

        // let (msg_commit, msg_welcome, info) = group.add_members(&provider, &keys, &[pkg]).unwrap();
        // group.merge_pending_commit(provider);
        // group.merge_staged_commit(provider, staged_commit);

        // StagedWelcome;

        // Self {
        //     channel_id,
        //     session_id,
        //     ciphersuite: todo!(),
        //     group,
        //     keys: todo!(),
        // }

        vec![]
    }

    // pub fn add_member(&mut self, channel_id: ChannelId, key_package: MlsKeyPackage) -> Result<()> {}

    // /// get or create the crypto state for a channel
    // // TODO: redo or remove this?
    // pub fn load_channel(&mut self, channel_id: ChannelId) -> &mut EncryptionChannel {
    //     self.channels.entry(channel_id).or_insert_with(|| {
    //         let config: ChannelEncryption = todo!();

    //         todo!()
    //     })
    // }

    // /// Get the cross-signing identity, if one has been initialized.
    // pub fn cross_signing(&self) -> Option<&CrossSigning> {
    //     todo!()
    // }

    // /// create a new blank cross signing state
    // pub fn init_cross_signing(&mut self) -> Result<(), Error> {
    //     todo!()
    // }

    // fn join_from_welcome(&mut self);

    pub fn handle_dispatch(&mut self, msg: E2EEDispatch) {
        // match msg {
        //     E2EEDispatch::Channel { channel_id, dispatch } => todo!(),
        //     E2EEDispatch::MlsKeyCount { user_id, session_id, count } => todo!(),
        //     E2EEDispatch::KeyshareRequest { sharer_id, nonce, request } => todo!(),
        //     E2EEDispatch::KeyshareResponse { recipient_id, nonce, response } => todo!(),
        // }

        // match msg {
        //     E2EEDispatch::Channel {
        //         channel_id,
        //         dispatch,
        //     } => todo!(),

        //     // 1. Create new ChannelCrypto from welcome
        //     // 2. Persist to storage
        //     // 3. Return "New Channel Joined" UI event
        //     E2EEDispatch::MlsWelcome {
        //         recipient_id,
        //         welcome,
        //     } => todo!(),

        //     // generate more keys
        //     E2EEDispatch::MlsKeyCount {
        //         user_id,
        //         session_id,
        //         count,
        //     } => todo!(),

        //     // send keyshare messages to the channel
        //     E2EEDispatch::KeyshareRequest {
        //         sharer_id,
        //         nonce,
        //         request,
        //     } => todo!(),
        //     E2EEDispatch::KeyshareResponse {
        //         recipient_id,
        //         nonce,
        //         response,
        //     } => todo!(),

        //     // send these to cross signing
        //     E2EEDispatch::IdentityUpdated { user_id, bundle } => todo!(),
        //     E2EEDispatch::SignatureAdded { user_id, signature } => todo!(),
        // }
    }

    // TODO: use this for rotation_period_ms
    // pub fn poll_output(&mut self) -> Option<Output> {
    //     todo!()
    // }
}

// pub struct KeyringData {
//     pub channel_id: ChannelId,
//     pub epochs: Vec<EpochKey>,
// }

// pub struct EpochKey {
//     pub epoch: MlsEpoch,
//     pub key: Vec<u8>,    // epoch exporter secret
//     pub nonce: Vec<u8>,  // for AES-GCM
// }

// mls exchanges symmetric keys, symmetric keys are used to actually encrypt

// TODO: indexeddb storage provider(?)
// impl StorageProvider<CURRENT_VERSION> for MemoryStorage {

// #[async_trait]
// pub trait EncryptionStorage: openmls_traits::storage::StorageProvider {
//     async fn save_cross_signing(&self, bundle: &CrossSigningPrivate) -> Result<()>;
//     async fn load_cross_signing(&self) -> Result<Option<CrossSigningPrivate>>;
//     async fn save_device_key(&self, key: &SignatureKeyPair) -> Result<()>;
//     async fn load_device_key(&self) -> Result<Option<SignatureKeyPair>>;
// }
