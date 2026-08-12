use lamprey::{
    v1::types::{
        Message, MessageDefaultMarkdownEncrypted, MessageEncrypted,
        e2ee::{E2EEDispatchChannel, MlsEpoch},
    },
    v2::types::SessionId,
};
use openmls::{
    framing::{MlsMessageBodyIn, MlsMessageIn},
    group::MlsGroup,
    prelude::{DeserializeBytes, OpenMlsProvider},
};

use crate::{manager::EncryptionShared, prelude::*};

pub struct EncryptionChannel {
    // channel_id: ChannelId,
    shared: Ref<EncryptionShared>,
    group: MlsGroup,
    // // encryption keys from older epochs
    // epoch_keys: HashMap<u64, EncryptionKey>,
    // epoch_keys: HashMap<MlsEpoch, EncryptionKey>,
}

impl EncryptionChannel {
    /// get the current mls epoch
    pub fn epoch(&self) -> MlsEpoch {
        let epoch = self.group.epoch();
        MlsEpoch(epoch.as_u64())
    }

    pub fn handle_dispatch(&mut self, msg: E2EEDispatchChannel) {

        // match msg {
        //     E2EEDispatchChannel::MlsKnock { key_package } => todo!(),

        //     // call handle_mls_message
        //     E2EEDispatchChannel::MlsMessage { sender_id, data } => {
        //         self.handle_mls_message(sender_id, &data)
        //     }
        // }
    }

    pub fn handle_mls_message(&mut self, sender_id: SessionId, data: &[u8]) {
        let msg = MlsMessageIn::tls_deserialize_exact_bytes(data).unwrap();
        match msg.extract() {
            MlsMessageBodyIn::PublicMessage(public_message_in) => todo!(),
            MlsMessageBodyIn::PrivateMessage(private_message_in) => todo!(),
            MlsMessageBodyIn::Welcome(welcome) => todo!(),
            MlsMessageBodyIn::GroupInfo(verifiable_group_info) => todo!(),
            MlsMessageBodyIn::KeyPackage(k) => {
                // self.group.add_members(
                //     &self.shared.provider,
                //     signer,
                //     &[k.validate(crypto, protocol_version)],
                // )
                todo!()
            }
        }
    }

    // OLD CODE BELOW
    // /// handle a e2ee dispatch message
    // pub fn handle_dispatch(&mut self, msg: E2EEMessage) -> Result<Option<Action>, Error> {
    //     match msg {
    //         E2EEMessage::MlsKnock {
    //             channel_id,
    //             key_package,
    //         } => {
    //             use openmls::prelude::tls_codec::{Deserialize, Serialize};

    //             let mut data_slice = key_package.data.as_slice();
    //             let kp_in = openmls::prelude::KeyPackageIn::tls_deserialize(&mut data_slice)
    //                 .map_err(|e| Error::MlsError(e.to_string()))?;

    //             let (commit, welcome, _group_info) = self.group.add_members(&*self.provider, &self.signer, &[kp_in.into()])
    //                 .map_err(|e| Error::MlsError(format!("{:?}", e)))?;

    //             self.group.merge_pending_commit(&*self.provider).map_err(|e| Error::MlsError(format!("{:?}", e)))?;

    //             let commit_msg = MlsCommitCreate {
    //                 data: commit.tls_serialize_detached().unwrap(),
    //             };
    //             let welcome_msg = MlsWelcomeCreate {
    //                 data: welcome.into_welcome().unwrap().tls_serialize_detached().unwrap(),
    //             };

    //             return Ok(Some(Action::AddMember(channel_id, commit_msg, welcome_msg)));
    //         }
    //         E2EEMessage::MlsMessage {
    //             sender_id,
    //             channel_id,
    //             data,
    //         } => {
    //             let mut data_slice = data.as_slice();
    //             let mls_in = MlsMessageIn::tls_deserialize(&mut data_slice).map_err(|e| Error::MlsError(e.to_string()))?;
    //             let proto = mls_in.try_into_protocol_message().map_err(|e| Error::MlsError(format!("{:?}", e)))?;
    //             let processed = self.group.process_message(&*self.provider, proto).map_err(|e| Error::MlsError(format!("{:?}", e)))?;

    //             match processed.into_content() {
    //                 ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
    //                     self.group
    //                         .merge_staged_commit(&*self.provider, *staged_commit).map_err(|e| Error::MlsError(format!("{:?}", e)))?;
    //                     // save group state
    //                 }
    //                 ProcessedMessageContent::ExternalJoinProposalMessage(_) => {}
    //                 _ => {}
    //             }
    //             return Ok(None);
    //         }
    //         _ => return Ok(None),
    //     }
    // }

    // /// Create an MLS commit to add a new member.
    // ///
    // /// Takes the new member's key package and returns the
    // /// serialized commit data to submit to the server.
    // pub fn create_add_commit(&mut self, key_package: &[u8]) -> Result<Vec<u8>, Error> {
    //     todo!()
    // }

    // /// Create an MLS welcome message for a new member.
    // ///
    // /// Returns the serialized welcome data to submit to the server.
    // pub fn create_welcome(&mut self) -> Result<Vec<u8>, Error> {
    //     todo!()
    // }

    // /// Generate a new MLS key package for upload.
    // ///
    // /// Returns the serialized key package data to upload to the server.
    // // NOTE: can i just have key
    // pub fn generate_key_package(&mut self) -> Result<Vec<u8>, Error> {
    //     todo!()
    // }
}

// maybe move actual encryption code somewhere else?
impl EncryptionChannel {
    // TODO: maybe add these?
    // fn get current encryption key
    // fn get current encryption key for epoch

    /// encrypt a message for the group.
    ///
    /// Derives the symmetric encryption key from the current MLS epoch's exported secret
    /// and uses standard AEAD (eg. AES-256-GCM) to encrypt the message payload.
    /// This allows caching old keys for history decryption without breaking MLS forward secrecy.
    pub fn encrypt_message(
        &self,
        msg: MessageDefaultMarkdownEncrypted,
    ) -> Result<MessageEncrypted> {
        let epoch = self.epoch();

        let secret =
            self.group
                .export_secret(self.shared.provider.crypto(), "lamprey-message", &[], 32)?;

        use aes_gcm::{
            Aes256Gcm, Key,
            aead::{Aead, KeyInit},
        };

        // TODO: fix deprecation warning
        // TODO: extract into fn derive_encryption_key(secret: &[u8]) -> Key<Aes256Gcm>
        let key = Key::<Aes256Gcm>::from_slice(&secret);
        let cipher = Aes256Gcm::new(key);
        let nonce = {
            let mut nonce = [0u8; 12];
            rand::fill(&mut nonce);
            nonce
        };

        let payload = serde_json::to_vec(&msg).unwrap();
        let ciphertext = cipher.encrypt((&nonce).into(), &*payload).unwrap();

        // FIXME: populate media_ids from attachments, embeds, components
        Ok(MessageEncrypted {
            ciphertext: ciphertext.into(),
            nonce: nonce.to_vec().into(),
            media_ids: vec![],
            epoch,
        })
    }

    /// decrypt a single message in place
    pub fn decrypt_message(&self, msg: &mut Message) -> Result<()> {
        self.decrypt_messages(core::slice::from_mut(msg))
    }

    /// decrypt messages in-place.
    ///
    /// Looks up MLS keys via the nonce, decrypts the ciphertext,
    /// and replaces the `MessageEncrypted` variant with the plaintext.
    pub fn decrypt_messages(&self, msgs: &mut [Message]) -> Result<()> {
        use lamprey::v1::types::MessageType;

        for msg in msgs {
            let encrypted = match &mut msg.latest_version.message_type {
                MessageType::Encrypted(m) => m,
                _ => continue,
            };

            // let key_data = self
            //     .epoch_keys
            //     .get(&encrypted.epoch.0)
            //     .ok_or(Error::KeyNotFound)?;

            // use aes_gcm::{
            //     Aes256Gcm, Key, Nonce,
            //     aead::{Aead, KeyInit},
            // };

            // let key = Key::<Aes256Gcm>::from_slice(&key_data.0);
            // let cipher = Aes256Gcm::new(key);
            // let nonce = Nonce::from_slice(&encrypted.nonce);

            // let decrypted_payload = cipher
            //     .decrypt(nonce, encrypted.ciphertext.as_ref())
            //     .map_err(|_| Error::DecryptionError)?;
            // let plaintext: lamprey_common::v1::types::message::MessageDefaultMarkdown =
            //     serde_json::from_slice(&decrypted_payload).map_err(|_| Error::DecryptionError)?;

            // msg.latest_version.message_type = MessageType::DefaultMarkdown(plaintext);

            todo!()
        }

        Ok(())
    }
}
