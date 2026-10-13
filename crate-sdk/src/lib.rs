//! client sdk for lamprey mountain
//!
//! You probably want to start with [`Client`], which provides handles to the various
//! subsystems:
//!
//! - [`http`]: typed rest api calls
//! - [`syncer`]: the live sync connection and its events
//! - [`cache`]: a client-side cache of server state
//! - [`messages`]: per-channel message fetching and caching
//! - [`flume`]: live-updating messages within a channel
//! - [`voice`]: realtime media handling for voice channels

#[cfg(feature = "cache")]
pub mod cache;
// pub mod cache_old;

#[cfg(feature = "flumes")]
pub mod flume;

#[cfg(feature = "voice")]
pub mod voice;

pub mod client;
mod error;
pub mod http;
pub mod messages;
pub mod syncer;

// TODO: impl these
// mod member_list;
// mod media; // media handling utilities
//
// #[cfg(feature = "document")]
// mod document;

pub use client::{Client, ClientBuilder};

pub(crate) mod prelude {
    pub use crate::error::Error;
    pub type Result<T, E = Error> = ::core::result::Result<T, E>;
}
