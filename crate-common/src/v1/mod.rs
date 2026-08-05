/// api types
pub mod types;

// TODO: don't require these features
#[cfg(all(feature = "serde", feature = "utoipa"))]
/// api http routes
pub mod routes;
