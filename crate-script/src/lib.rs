pub mod engine;
pub mod error;
pub mod limits;

#[cfg(feature = "javascript")]
pub mod javascript;

#[cfg(feature = "wasm")]
pub mod wasm;

pub use engine::{Engine, ExecutionHandle, Executor};
pub use error::{Error, Result};
pub use limits::Limits;

// TODO: automatically put runs to sleep to save memory
// TODO: automatically awaken runs when triggered

#[cfg(test)]
mod tests;
