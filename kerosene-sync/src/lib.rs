//! websocket sync

pub mod permissions;
pub mod queue;
pub mod transport;
pub mod util;

// TODO: implement these?
// pub mod error;
// pub mod actor;
// pub mod connection;
// pub mod handshake;
// pub mod subscriptions;

pub(crate) mod prelude {
    pub use lamprey_backend_core::prelude::{Error, Result};
    // TODO: pub use kerosene_core::prelude::*;
    pub type WsMessage = axum::extract::ws::Message;
}
