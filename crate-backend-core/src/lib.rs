pub use kerosene_core::{config, ffmpeg, types};
pub mod data;
pub mod error;
pub mod queue;

pub use error::{Error, Result};

/// common types used everywhere in backend
pub mod prelude {
    pub use crate::error::{AnyErrorExt, Error, LegacyErrorExt, Result};
    pub use kerosene_core::prelude::*;
}
