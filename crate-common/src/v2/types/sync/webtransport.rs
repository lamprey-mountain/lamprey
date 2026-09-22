// TODO: impl and use
// NOTE: move this to a separate stream module?

/// A protocol that can be used with a stream
///
/// Syncing takes place over multiple streams. Each stream can have a different
/// protocol. This defines a protocol that can be used with a stream.
pub trait StreamProtocol {
    /// the initial message sent when opening this stream
    type Initial;

    /// what the client sends
    type Command;

    /// what the server sends
    type Event;

    /// whether commands and events should be sent via datagram if possible
    ///
    /// a standard stream is opened to send initial through and to control the
    /// lifetime of the stream. ie. when that stream is closed, datagrams sent to that
    /// stream should be dropped.
    fn is_datagram() -> bool {
        false
    }

    /// whether messages sent via this stream
    fn is_compressed() -> bool {
        true
    }
}

/// a handshake to open a stream
///
/// sent by the client
pub enum Handshake {
    Sync(sync::Initial),
    // TODO: subscriptions, voice(?), etc
}

/// protocol for the main sync stream
///
/// this stream must be created before anything else and is used for authentication
// TODO: have a separate `identify` protocol, make `sync` only handle syncing?
pub mod sync {
    pub use crate::v2::types::sync::{SyncIdentify as Identify, SyncResume as Resume};
    use crate::{v1::types::presence::Presence, v2::types::sync::webtransport::StreamProtocol};

    pub struct Protocol;

    pub enum Initial {
        Identify(Identify),
        // Shard(Shard), // use multiple websocket connections
        Resume(Resume),
    }

    pub enum Command {
        PresenceUpdate {
            presence: Presence,
        },

        Connect {
            // add this command for websockets? or use Initial::Shard?
        },

        Close,
        Pong,
    }

    pub enum Event {
        Ping,
        Resumed,

        Dispatch {
            // seq: u64,
            // dispatch: Box<Dispatch>,
        },

        Reconnect {
            // can_resume: bool,
        },

        Error {
            // error: String,
            // code: SyncErrorCode,
        },
    }

    impl StreamProtocol for Protocol {
        type Initial = Initial;
        type Command = Command;
        type Event = Event;
    }
}

// NOTE: do i include basic stuff (heartbeats, resuming, errors) for every protocol or just sync?
