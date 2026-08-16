pub mod bot;
pub mod commands;
pub mod config;
pub mod database;
pub mod duration;

pub(crate) mod prelude {
    pub use anyhow::Result;
}
