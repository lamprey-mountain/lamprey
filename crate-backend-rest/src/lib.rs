pub mod endpoints;
pub mod util;

pub(crate) mod prelude {
    pub(crate) use crate::util::{Globals, Req, Routes, routes::Handlers};
    pub(crate) use common::v1::routes;
    pub(crate) use lamprey_backend_core::prelude::*;
    pub(crate) use lamprey_macros::handlers_new as handlers;
    pub(crate) use validator::Validate;
}
