use std::time::Duration;

use tokio::time::Instant;

/// send a heartbeat every so often
pub const HEARTBEAT_TIME: Duration = Duration::from_secs(30);

/// if a pong isnt received after this time, close the connection
pub const CLOSE_TIME: Duration = Duration::from_secs(10);

/// the maximum number of events to retain in the queue before killing the connection
// TODO: decide how this should work with webtransport streams. letting EVERY stream have MAX_QUEUE_LEN events could result in excessive memory usage...
pub const MAX_QUEUE_LEN: usize = 256;

/// utility to calculate deadlines for connection health checks.
#[derive(Debug, Clone, Copy)]
pub enum Timeout {
    /// when the server will next send a `Ping`
    Ping(Instant),

    /// the client must respond with a `Pong` before this deadline, otherwise the connection will be closed.
    Close(Instant),
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

/// status of a connection close
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionClose {
    Clean,
    Dirty,
}

impl ConnectionClose {
    pub fn from_ws_code(code: u16) -> Self {
        // 1000 = Normal Closure, 1001 = Going Away
        if code == 1000 || code == 1001 {
            Self::Clean
        } else {
            Self::Dirty
        }
    }

    pub fn is_clean(&self) -> bool {
        self == &Self::Clean
    }
}
