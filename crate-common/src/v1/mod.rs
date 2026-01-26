/// api types
pub mod types;

// TODO: don't require utoipa
#[cfg(any())]
#[cfg(feature = "utoipa")]
/// api http routes
pub mod routes;
