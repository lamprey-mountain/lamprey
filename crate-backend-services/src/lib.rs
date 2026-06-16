pub mod globals;
pub mod services;
pub mod util;

pub(crate) mod prelude {
    pub use crate::globals::Globals;
    pub use lamprey_backend_core::prelude::*;
    pub use std::sync::Arc;
}
