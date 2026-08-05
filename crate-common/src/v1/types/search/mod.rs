use lamprey_macros::record;

// TODO: group types by resource type rather than request/response

pub mod request;
pub mod response;
pub mod stats;

pub use request::*;
pub use response::*;

/// what order to return search results in
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum Order {
    #[default]
    #[serde(rename = "asc")]
    Ascending,

    #[serde(rename = "desc")]
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
