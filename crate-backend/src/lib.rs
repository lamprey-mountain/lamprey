pub mod routes;
pub mod server;
pub mod services;
pub mod state;
pub mod sync;

// TODO: remove, merge into mod server
pub mod serve;

// TODO: remove, merge logic into lamprey-cli instead?
pub mod cli;

// TODO: remove most of these, allow setting limits in config?
pub mod consts;

// NOTE: unsure what to do with this
pub mod metrics;

// TODO: remove these
pub mod config;
pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use state::{ServerState, ServerStateInner};

pub(crate) mod prelude {
    pub use std::sync::Arc;

    pub use crate::state::Globals;
    pub use bytes::Bytes;
    pub use lamprey_backend_core::prelude::*;

    pub type CoreResult<T, E> = ::core::result::Result<T, E>;

    pub use futures_util::StreamExt;
}
