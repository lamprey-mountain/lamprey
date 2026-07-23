// TODO: move here: enum ConnectionErrorSeverity, fn severity

// TODO: errors for connections
// enum ConnectionError {}

use lamprey::v1::types::error::SyncErrorCode;
// TEMP: reexport?
pub use lamprey_backend_core::Error;

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    // TODO
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionErrorSeverity {
    /// client is fine to continue running
    Notice,

    /// client needs to reconnect
    Reconnect,

    /// client needs to start a new connection from scratch
    Fatal,
    // separate "can't reconnect at all" severity for stuff like eg. logouts?
}

fn severity(err: &Error) -> ConnectionErrorSeverity {
    use ConnectionErrorSeverity::*;
    match err {
        Error::SyncError(c) => match c {
            SyncErrorCode::InvalidSeq => Reconnect,
            SyncErrorCode::Timeout => Reconnect,
            SyncErrorCode::Unauthorized => Notice,
            SyncErrorCode::Unauthenticated => Notice,
            SyncErrorCode::AlreadyAuthenticated => Notice,
            SyncErrorCode::AuthFailure => Fatal,
            SyncErrorCode::InvalidData => Fatal,
        },

        // TODO: correct severities for more errors
        _ => Fatal,
    }
}
