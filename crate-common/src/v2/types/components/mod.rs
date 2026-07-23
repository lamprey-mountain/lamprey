pub mod acl;
pub mod action;
pub mod builder;
pub mod impls;
pub mod interactive;
pub mod types;
pub mod validate;

#[cfg(any())]
pub mod cursor;

// TODO: rename?
pub use crate::v1::types::components::ComponentCustomId;

// TODO: rename?
pub use crate::v1::types::components::ComponentId;

pub use types::{Component, ComponentType, Components};

#[cfg(feature = "serde")]
mod _serde {
    // TODO: text or struct for ComponentCreate
}

#[cfg(feature = "utoipa")]
mod _utoipa {
    // TODO: text or struct for ComponentCreate - maybe can be done with utoipa attrs instead of manual impl?
}

#[cfg(test)]
mod tests;
