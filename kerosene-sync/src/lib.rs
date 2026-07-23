//! websocket sync

pub mod connection;
pub mod error;
pub mod permissions;
pub mod queue;
pub mod subscriptions;
pub mod transport; // TODO: share with lamprey-sdk (maybe put in common?)
pub mod util;

// pub(crate) mod prelude {
//     pub use errors, etc
//     type WsMessage = axum::extract::ws::Message;
// }

// TODO: create `mod actor`, move routes/sync.rs logic there(?)
