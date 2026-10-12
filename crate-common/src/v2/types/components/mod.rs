//! components are a way to layout stuff

pub mod impls;
mod inner;
pub mod interactive;
pub mod types;
pub mod validate;

// TODO: implement transform
// pub mod transform;

pub use crate::v1::types::components::{ComponentCustomId, ComponentId};
pub use inner::{Component, ComponentMedia, Components, Create};
pub use types::ComponentType;

#[cfg(test)]
mod tests;
