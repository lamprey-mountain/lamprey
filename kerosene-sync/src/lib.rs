//! sync stuff

pub mod v1;
pub mod v2;

// TEMP: compat
pub use v1::*;

// TODO: implement these?
// pub mod error;
// pub mod actor;
// pub mod connection;
// pub mod handshake;
// pub mod subscriptions;

pub(crate) mod prelude {
    pub use kerosene_core::prelude::*;
    pub use lamprey_backend_core::prelude::{Error, Result}; // TEMP: use kerosene instead!
    pub type WsMessage = axum::extract::ws::Message;
}
