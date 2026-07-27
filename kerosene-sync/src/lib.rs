//! websocket sync

pub mod error;
pub mod permissions;
pub mod queue;
pub mod transport;
pub mod util;

// TODO: remove these?
// pub mod connection_old;
// pub mod subscriptions_old;

// TODO: implement these?
pub mod actor;
pub mod connection;
pub mod handshake;
pub mod subscriptions;

pub(crate) mod prelude {
    pub use lamprey_backend_core::prelude::{Error, Result};
    pub type WsMessage = axum::extract::ws::Message;
}
