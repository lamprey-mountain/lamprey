pub mod globals;
pub mod services;
pub mod util;

pub(crate) mod prelude {
    pub use crate::globals::{Globals, GlobalsOwned};
    pub use crate::services::Services;
    pub use bytes::Bytes;
    pub use lamprey_backend_core::prelude::*;
    pub use std::sync::Arc;
    pub type CoreResult<T, E> = ::core::result::Result<T, E>;
    pub use futures_util::StreamExt;
}

// TEMP: compatability
pub mod compat;
pub(crate) use compat::*;
