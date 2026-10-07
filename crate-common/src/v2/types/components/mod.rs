//! components are a way to layout stuff

// TODO: rearrange mod components, impls
// components::components::Components is kinda bad
// i should probably have a module for each component type

pub mod components;
pub mod impls;
pub mod validate;

// TODO: implement transform
// pub mod transform;

// TODO: merge mod acl, action into interactive?
pub mod acl;
pub mod action;
pub mod interactive;

pub use crate::v1::types::components::{ComponentCustomId, ComponentId};
pub use components::{Component, ComponentType, Components};

#[cfg(test)]
mod tests;
