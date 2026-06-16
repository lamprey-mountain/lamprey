pub mod endpoints;
pub mod util;

pub(crate) mod prelude {
    pub use crate::util::{Globals, Req, Routes, routes::Handlers};
    pub use lamprey_backend_core::prelude::*;
    pub use lamprey_macros::handlers_new as handlers;
}
