pub mod actor;
pub mod config;
pub mod database;
pub mod platform;
pub mod types;
pub mod util;

// TODO: remove
pub mod bridge_old;

pub(crate) mod prelude {
    // TODO: maybe use a custom error type?
    pub type Error = anyhow::Error;
    pub type Result<T> = ::core::result::Result<T, anyhow::Error>;

    pub use serde::{Deserialize, Serialize};
    pub use std::sync::Arc;
    pub use url::Url;

    pub use crate::bridge_old; // TEMP: compat
    pub use crate::platform::{discord, lamprey};
    pub use crate::types::{MessageId, PortalId, RealmId};
}
