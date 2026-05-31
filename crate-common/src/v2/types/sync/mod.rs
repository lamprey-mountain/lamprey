// TODO: copy SyncParams here

use url::Url;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::{
    v1::types::{
        application::Application,
        document::DocumentUpdate,
        error::SyncErrorCode,
        presence::Presence,
        voice::{
            messages::{SignallingCommand, SignallingEvent},
            VoiceState, VoiceStateUpdate,
        },
        ChannelId, ConnectionId, DocumentBranchId, Session, SessionId, SessionToken, User, UserId,
    },
    v2::types::{
        media::Media,
        sync::{
            channel::DispatchChannel,
            invite::DispatchInvite,
            room::DispatchRoom,
            shard::Syncer,
            subscribe::{DispatchSubscriptions, SyncSubscriptionsUpdate},
            user::DispatchUser,
            webhook::DispatchWebhook,
        },
        ShardId, SyncId,
    },
};

pub mod channel;
pub mod filter;
pub mod invite;
pub mod room;
pub mod shard;
pub mod subscribe;
pub mod user;
pub mod visibility;
pub mod webhook;

pub use crate::v1::types::{SyncCompression, SyncFormat as SyncEncoding, SyncVersion};

/// query parameters when establishing a websocket sync connection
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct WebsocketSyncParams {
    pub version: SyncVersion,

    pub compression: Option<SyncCompression>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub encoding: SyncEncoding,
}

/// a command from the client to the sync worker
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "op"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncCommand {
    /// start a new sync connection
    Identify {
        token: SessionToken,
        presence: Option<Presence>,
        properties: SyncProperties,
    },

    /// connect to an existing connection
    ///
    /// this includes connecting as a shard
    Resume {
        token: SessionToken,
        connection_id: ConnectionId,
        seq: u64,

        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        shard_id: Option<ShardId>,
    },

    /// cleanly close this stream or sync
    ///
    /// closes the entire sync connection if this is the main transport
    Close,

    /// heartbeat
    // NOTE: do i want to reverse heartbeats to be able to detect backpressure?
    Pong,

    /// set presence
    PresenceUpdate { presence: Presence },

    /// initialize a new voice connection
    VoiceConnect {
        voice_state: Box<VoiceStateUpdate>,
        nonce: Option<String>,
    },

    /// dispatch a command to a voice connection
    VoiceDispatch {
        channel_id: ChannelId,
        nonce: Option<String>,
        command: Box<SignallingCommand>,
    },

    /// edit a document
    ///
    /// must be subscribed via DocumentSubscribe
    DocumentEdit {
        /// the document thats being edited
        channel_id: ChannelId,

        branch_id: DocumentBranchId,

        /// the encoded update to this document
        update: Box<DocumentUpdate>,
    },

    /// update your document presence
    ///
    /// must be subscribed via DocumentSubscribe
    DocumentPresence {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        // TODO: strongly type these
        cursor_head: String,
        cursor_tail: Option<String>,
    },

    /// subscribe to some resources
    Subscribe(SyncSubscriptionsUpdate),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct SyncProperties {
    /// a human readable note
    ///
    /// this is for the service operator to read when debugging
    pub note: Option<String>,

    // metadata for the library used to connect
    pub library_commit: Option<String>,
    pub library_version: Option<String>,
    pub library_name: Option<String>,
    pub library_url: Option<Url>,

    // metadata for the client/application itself
    pub application_commit: Option<String>,
    pub application_version: Option<String>,
    pub application_name: Option<String>,
    pub application_url: Option<Url>,
}

/// an event from the sync worker to the client
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "op"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncEventEnvelope {
    /// heartbeat
    Ping,

    /// all missed messages have been sent, you are now tailing the live event stream
    Resumed,

    /// data to keep local copy of state in sync with server
    Dispatch {
        /// the connection sequence number of this event, for resuming
        seq: u64,

        /// the sync dispatch itself
        dispatch: Box<Dispatch>,

        /// the nonce for responses
        ///
        /// set if:
        ///
        /// - this is in response to a http request with the `Idempotency-Key` header set
        /// - this is in response to a `SyncCommand` with an associated nonce
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        nonce: Option<String>,
    },

    /// client needs to reconnect
    Reconnect {
        /// whether the client can resume
        can_resume: bool,
    },

    /// an error occured
    Error { error: String, code: SyncErrorCode },
}

/// an event from the sync worker to a webhook
///
/// the webhook must respond with a 2xx status code within 3 seconds
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "op"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncEventEnvelopeWebsocket {
    /// heartbeat
    ///
    /// webhook should respond with 204 no content. make sure to validate that the signature is correct.
    Ping,

    // NOTE: is this needed for webhooks
    Resumed,

    /// a dispatch
    Dispatch {
        /// the connection sequence number of this event, for resuming
        seq: u64,

        /// the sync dispatch itself
        dispatch: Box<Dispatch>,

        /// the nonce for responses
        ///
        /// set if this is in response to a http request with the `Idempotency-Key` header set
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        nonce: Option<String>,
    },

    /// an error occured
    ///
    /// client may need to re-setup the connection
    Error {
        error: String,
        code: SyncErrorCode,
    },
}

/// something happened
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Dispatch {
    /// successfully connected
    Ready {
        /// current user, null if session is unauthed
        user: Option<Box<User>>,

        /// the associated application
        ///
        /// - if an application is connecting on behalf of a user, this is the application that is connecting.
        /// - if a bot is connecting, this is the application the bot belongs to
        application: Option<Box<Application>>,

        /// current session
        session: Box<Session>,

        /// the syncer object
        syncer: Box<Syncer>,

        /// the id of this shard, if this is a sharded connection
        shard_id: Option<ShardId>,
    },

    /// extra data for the client to function, sent after Ready
    Ambient {
        /// the sync id that this Ambient message is for
        sync_id: SyncId,
        // TODO: what goes here? returning all rooms could be bad for large bots
        // /// all rooms the user can see
        // rooms: Vec<Room>,

        // /// all roles in all rooms the user can see
        // roles: Vec<Role>,

        // /// all non-thread channels the user can see
        // channels: Vec<Channel>,

        // /// all active (ie. not archived) threads the user can see
        // threads: Vec<Channel>,

        // /// the user's room member object for each room the user is in
        // room_members: Vec<RoomMember>,

        // /// user's global preferences
        // config: PreferencesGlobal,
        // NOTE: maybe i should include even more data
        // - friends/relationships (including friend requests)
        // - dms
        // - emoji
    },

    /// receive a signalling message from a voice server
    VoiceDispatch {
        /// who to send this dispatch to
        user_id: UserId,
        channel_id: ChannelId,
        payload: Box<SignallingEvent>,
    },

    // TODO: redesign this type
    /// a voice state was updated
    VoiceState {
        /// the id of the user who's voice state was updated
        user_id: UserId,
        state: Option<Box<VoiceState>>,

        // HACK: make it possible to use this for auth checks
        #[cfg_attr(feature = "serde", serde(skip))]
        old_state: Option<Box<VoiceState>>,
    },

    /// an edit to a document
    ///
    /// only returned if subscribed
    DocumentEdit {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,

        /// the encoded update to this document
        update: Box<DocumentUpdate>,
    },

    /// user presence in a document
    ///
    /// only returned if subscribed
    DocumentPresence {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        user_id: UserId,

        // TODO: strongly type
        cursor_head: String,
        cursor_tail: Option<String>,
    },

    /// A piece of media has processed and is now in the `Uploaded` state.
    MediaProcessed {
        session_id: SessionId,
        media: Box<Media>,
    },

    MediaUpdate {
        media: Box<Media>,
    },

    #[cfg(feature = "feat_e2ee")]
    EncryptionDispatch {
        /// who to send this dispatch to
        user_id: UserId,
        payload: E2EEMessage,
    },

    // TODO: add
    // SyncCreate,
    // SyncDelete,
    // ShardCreate,
    // ShardDelete,
    #[serde(untagged)]
    Room(DispatchRoom),

    #[serde(untagged)]
    Channel(DispatchChannel),

    #[serde(untagged)]
    User(DispatchUser),

    #[serde(untagged)]
    Subscriptions(DispatchSubscriptions),

    #[serde(untagged)]
    Invite(DispatchInvite),

    #[serde(untagged)]
    Webhook(DispatchWebhook),

    // TODO: add federation sync events
}
