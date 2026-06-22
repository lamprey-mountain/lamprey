pub mod config;
pub mod data;
pub mod queue;
pub mod error;
pub mod types;

pub use error::{Error, Result};

/// common types used everywhere in backend
pub mod prelude {
    pub use crate::error::{Error, Result};
}
