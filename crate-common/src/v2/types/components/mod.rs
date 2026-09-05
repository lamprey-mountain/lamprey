pub mod acl;
pub mod action;
pub mod components; // TODO: rename? components::components::Components is kinda bad
mod error; // TODO: make these public?
pub mod impls;
pub mod interactive;
pub mod validate;

// TODO: impl and use this instead of flume delta
// pub mod delta;

// TODO: remove these?
// pub mod builder;
// pub mod tree;

// NOTE: maybe rename id types to be more clear? probably not.
pub use crate::v1::types::components::{ComponentCustomId, ComponentId};

pub use components::{Component, ComponentType, Components};

#[cfg(test)]
mod tests;
