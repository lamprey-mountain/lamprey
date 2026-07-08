//! websocket sync

use common::v1::types::error::SyncErrorCode;

pub mod error;
pub mod next; // TODO: rename?
pub mod permissions; // TODO: lift to lamprey-common
pub mod queue;
pub mod subscriptions;
pub mod transport; // TODO: share with lamprey-sdk (maybe put in common?)
pub mod util;

use crate::error::Error;

type WsMessage = axum::extract::ws::Message;

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

// TODO: create `mod actor`, move routes/sync.rs logic there
