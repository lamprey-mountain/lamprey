//! components are a way to style and layout stuff

pub mod impls; // TODO: inline impls
mod inner;
pub mod interactive;
pub mod transform;
pub mod types;
pub mod validate;

pub use crate::v1::types::components::{ComponentCustomId, ComponentId};
pub use inner::{Component, ComponentMedia, Components, Create};
pub use types::ComponentType;

#[cfg(test)]
mod tests;
