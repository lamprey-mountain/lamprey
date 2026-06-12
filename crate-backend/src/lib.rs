pub mod cli;
pub mod config;
pub mod consts;
pub use lamprey_backend_data_postgres as data;
pub mod error;
pub mod metrics;
pub mod routes;
pub mod serve;
pub mod services;
pub mod state;
pub mod sync;
pub mod types;

pub use error::{Error, Result};
pub use state::{ServerState, ServerStateInner};

pub(crate) mod prelude {
    pub use bytes::Bytes;
    pub use lamprey_backend_core::prelude::*;

    pub type CoreResult<T, E> = ::core::result::Result<T, E>;
}
