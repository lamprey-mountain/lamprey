use lamprey_macros::record;

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

/// generic search request struct
#[record]
pub struct SearchRequest {
    /// the full text search query.
    #[schema(required = false, min_length = 1, max_length = 2048)]
    #[validate(length(min = 1, max = 2048))]
    #[serde(default)]
    pub query: Option<String>,

    /// sort order (ascending/descending)
    #[serde(default = "Order::descending")]
    pub sort_order: Order,

    /// the maximum number of items to return
    #[serde(default = "default_limit")]
    #[schema(default = 100, minimum = 0, maximum = 1024)]
    #[validate(range(min = 0, max = 1024))]
    pub limit: u16,

    /// the number of items to skip before returning
    #[serde(default)]
    #[schema(default = 0, minimum = 0, maximum = 65535)]
    #[validate(range(min = 0, max = 65535))]
    pub offset: u16,
}

pub const fn default_limit() -> u16 {
    100
}
