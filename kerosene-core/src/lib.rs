// TEMP: proxying for now?
// TODO: write out this crate
pub use lamprey_backend_core::config;
// pub use lamprey_backend_core::queue;
pub use lamprey_backend_core::types;

pub mod database;
pub mod error;

/// common types used everywhere in backend
pub mod prelude {
    pub use crate::error::{ApiError, ApiResult, ServerError, ServerResult};
}
