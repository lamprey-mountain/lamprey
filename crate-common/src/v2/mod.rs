pub mod migrations;
pub mod types;

// TODO: don't require these features
#[cfg(all(feature = "serde", feature = "utoipa"))]
pub mod routes;
