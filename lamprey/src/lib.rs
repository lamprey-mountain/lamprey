/// # lamprey mountain documentation
///
/// todo: write more stuff here
pub mod docs {
    #[doc = include_str!("../../docs/src/permissions.md")]
    pub mod sync {}

    #[doc = include_str!("../../docs/src/permissions.md")]
    pub mod permissions {}
}

/// # kerosene documentation
///
/// Kerosene is the canonical backend implementation for lamprey mountain.
#[cfg(any())]
pub mod docs_kerosene {}
