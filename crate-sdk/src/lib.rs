#[cfg(feature = "cache")]
pub mod cache;
// pub mod cache_old;

#[cfg(feature = "flumes")]
pub mod flume;

#[cfg(feature = "voice")]
pub mod voice; // NOTE: unsure if i should make this a pub mod?

pub mod client;
mod error;
pub mod http;
pub mod messages;
pub mod syncer;

// TODO: impl these
// mod member_list;
//
// #[cfg(feature = "document")]
// mod document;

pub use client::{Client, ClientBuilder};

pub(crate) mod prelude {
    pub use crate::error::Error;
    pub type Result<T> = ::core::result::Result<T, Error>;
}
