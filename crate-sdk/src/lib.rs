pub mod cache;
pub mod client;
mod error;
pub mod http;
mod member_list;
pub mod messages;
pub mod syncer;

#[cfg(feature = "voice")]
mod voice;

#[cfg(feature = "document")]
mod document;

pub use client::{Client, ClientBuilder};

pub(crate) mod prelude {
    pub use crate::error::Error;
    pub type Result<T> = ::core::result::Result<T, Error>;
}
