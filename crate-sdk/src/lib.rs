#[cfg(feature = "cache")]
pub mod cache;
// pub mod cache_old;

#[cfg(feature = "flumes")]
pub mod flume;

#[cfg(feature = "voice")]
pub mod voice; // NOTE: unsure if i should make this a pub mod?

#[cfg(feature = "document")]
mod document;

pub mod client;
mod error;
pub mod http;
mod member_list;
pub mod messages;
pub mod syncer;

pub use client::{Client, ClientBuilder};

pub(crate) mod prelude {
    pub use crate::error::Error;
    pub type Result<T> = ::core::result::Result<T, Error>;
}
