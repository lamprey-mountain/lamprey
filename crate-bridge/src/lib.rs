pub mod bridge;
pub mod config;
pub mod database;
pub mod util;

pub mod discord;
pub mod lamprey;

pub(crate) mod prelude {
    // NOTE: maybe use a custom error type
    pub type Error = anyhow::Error;
    pub type Result<T> = ::core::result::Result<T, anyhow::Error>;
    pub use crate::{bridge, discord, lamprey};
    pub use std::sync::Arc;
}
