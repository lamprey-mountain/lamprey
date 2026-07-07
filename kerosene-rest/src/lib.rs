mod endpoints;
mod util;

pub use util::auth::{Auth, Identity};
pub use util::request::Req;
pub use util::routes::Routes;

pub(crate) mod prelude {
    pub(crate) use crate::util::{Globals, Req};
    pub(crate) use common::util::routes::Endpoint;
    pub(crate) use common::v1::routes;
    pub(crate) use kerosene_core::error::ServerError as Error;
    pub(crate) use kerosene_core::error::ServerResult as Result;
    pub(crate) use kerosene_core::prelude::*;
    pub(crate) use lamprey_macros::handler_new as handler;
    pub(crate) use validator::Validate;
}
