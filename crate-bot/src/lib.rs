//! A general purpose bot.
//!
//! This is mostly meant to show off how to use various parts of the api rather
//! than for any practical purposes...

pub mod bot;
pub mod commands;
pub mod config;
pub mod llm;
pub mod reminders;
pub mod store;
pub mod util;

pub(crate) mod prelude {
    pub use anyhow::Result;
}
