#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

pub mod request;
pub mod response;
pub mod stats;

pub use request::*;
pub use response::*;

/// what order to return search results in
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Order {
    #[default]
    #[cfg_attr(feature = "serde", serde(rename = "asc"))]
    Ascending,

    #[cfg_attr(feature = "serde", serde(rename = "desc"))]
    Descending,
}

impl Order {
    pub fn descending() -> Order {
        Order::Descending
    }

    pub fn ascending() -> Order {
        Order::Ascending
    }
}
