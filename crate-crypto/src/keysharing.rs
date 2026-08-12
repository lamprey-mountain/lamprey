// TODO: implement this? maybe merge into mod encryption?

// use lamprey_common::v1::types::e2ee::{KeyshareRequest, MlsEpoch};

// use crate::{ChannelCrypto, error::Error};

// impl ChannelCrypto {
//     /// Create a request for historical encryption keys.
//     ///
//     /// Specifies the starting epoch and the maximum number of
//     /// epochs to request.
//     pub fn keyshare_create_request(
//         &self,
//         start_epoch: MlsEpoch,
//         limit: u8,
//     ) -> Result<KeyshareRequest, Error> {
//         todo!()
//     }

//     /// Create a response to a keyshare request.
//     ///
//     /// Encrypts the requested historical keyring data and returns
//     /// the encrypted blob to send back.
//     pub fn keyshare_create_response(&self, request: &KeyshareRequest) -> Result<Vec<u8>, Error> {
//         todo!()
//     }

//     /// Import historical encryption keys from a keyshare response.
//     ///
//     /// Decrypts and stores the keyring data so past messages
//     /// can be decrypted.
//     pub fn keyshare_import(&mut self, keyring_data: &[u8]) -> Result<(), Error> {
//         todo!()
//     }
// }
