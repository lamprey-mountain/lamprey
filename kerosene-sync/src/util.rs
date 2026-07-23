use std::time::Duration;

use lamprey::v1::types::Session;
use tokio::time::Instant;

/// send a heartbeat every so often
pub const HEARTBEAT_TIME: Duration = Duration::from_secs(30);

/// if a pong isnt received after this time, close the connection
pub const CLOSE_TIME: Duration = Duration::from_secs(10);

/// the maximum number of events to retain in the queue before killing the connection
pub const MAX_QUEUE_LEN: usize = 256;

// TODO: remove
/// where this connection is in the handshake
#[derive(Debug, Clone)]
pub enum ConnectionState {
    /// not yet authenticated; waiting for a `Hello` message
    Unauthed,

    /// successfully authenticated to this session
    Authenticated { session: Session },

    /// was authenticated to this session, but is no longer connected
    Disconnected { session: Session },
}

/// utility to calculate deadlines for connection health checks.
pub enum Timeout {
    /// when the server will next send a `Ping`
    Ping(Instant),

    /// the client must respond with a `Pong` before this deadline, otherwise the connection will be closed.
    Close(Instant),
}

impl ConnectionState {
    pub fn session(&self) -> Option<&Session> {
        match self {
            ConnectionState::Unauthed => None,
            ConnectionState::Authenticated { session } => Some(session),
            ConnectionState::Disconnected { session } => Some(session),
        }
    }
}

impl Timeout {
    pub fn for_ping() -> Self {
        Timeout::Ping(Instant::now() + HEARTBEAT_TIME)
    }

    pub fn for_close() -> Self {
        Timeout::Close(Instant::now() + CLOSE_TIME)
    }

    pub fn get_instant(&self) -> Instant {
        match self {
            Timeout::Ping(instant) => *instant,
            Timeout::Close(instant) => *instant,
        }
    }
}
