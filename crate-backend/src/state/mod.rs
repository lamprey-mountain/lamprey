//! global server state

#[cfg(any())]
mod queue;

// TEMP: reexport
pub use kerosene_services::globals::Globals;
pub use kerosene_services::globals::server_state::{
    MessageBroadcastInner, ServerState, ServerStateInner,
};

pub mod messaging {
    pub use kerosene_services::globals::messaging::*;
}
