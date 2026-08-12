use lamprey_macros::record;

use crate::v1::types::{
    ChannelId, SessionId, UserId, e2ee::cross_signing::CrossSigningBundle, misc::binary::Binary,
};

#[record]
#[serde(tag = "op")]
pub enum Dispatch {
    Channel {
        channel_id: ChannelId,
        // #[serde(flatten, untagged)] // maybe?
        dispatch: DispatchChannel,
    },

    /// cross signing identity updated
    IdentityReplaced {
        user_id: UserId,
        bundle: CrossSigningBundle,
    },
    // NOTE: probably not needed?
    // /// signature received for a cross signing key
    // IdentitySigned {
    //     // user_id: UserId,
    //     // signature: CrossSigningSignature,
    // },
}

#[record]
#[serde(tag = "op")]
pub enum DispatchChannel {
    /// a mls protocol message
    ///
    /// eg. commit, proposal, or application data
    Mls {
        /// the session that authored this message
        sender_id: SessionId,

        /// the opaque mls ProtocolMessage bytes
        // TODO: find an appropriate size limit for this
        data: Binary<4194304>,
    },
    // TODO: add these
    // KeyshareRequest,
    // KeyshareResponse,
}

#[cfg(any())]
mod old {
    #[record]
    #[serde(tag = "type")]
    pub enum E2EEDispatch {
        Channel {
            channel_id: ChannelId,
            dispatch: E2EEDispatchChannel,
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
    }

    /// a channel-specific dispatch
    #[record]
    #[serde(tag = "type")]
    pub enum E2EEDispatchChannel {
        /// someone wants to join a channel
        ///
        /// - a commit should be generated to allow this person to join
        /// - sent to one person in the group
        /// - prioritizes sending to a session from the same user
        MlsKnock {
            /// the key package of the person who wants to join
            // the server grabs a random mls key package and sends it here
            key_package: MlsKeyPackage,
        },

        // /// a new device has been added to one or more channels
        // ///
        // /// sent to the one session that is joining
        // MlsWelcome {
        //     recipient_id: SessionId,

        //     #[serde(flatten)]
        //     welcome: MlsWelcome,
        // },

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

            /// the opaque mls ProtocolMessage bytes
            // TODO: find an appropriate size limit for this
            data: Binary<4194304>,
        },
    }
}
