//! sync stuff

pub mod error;
pub mod v1;
pub mod v2;

// TEMP: compat
pub use v1::*;

// TODO: implement these?
// pub mod actor;
// pub mod connection;
// pub mod handshake;
// pub mod subscriptions;

pub(crate) mod prelude {
    pub use crate::error::{Error, Result};
    pub use kerosene_core::prelude::*;
    pub type WsMessage = axum::extract::ws::Message;
}
