use axum::routing::MethodFilter;
use common::util::routes::Method;

pub mod auth;
pub mod request;
pub mod routes;

pub use auth::Auth;
use lamprey_backend_core::config::Config;
pub use request::Req;
pub use routes::Routes;

pub(crate) trait MethodExt {
    fn to_filter(&self) -> MethodFilter;
}

impl MethodExt for Method {
    fn to_filter(&self) -> MethodFilter {
        match self {
            Method::Get => MethodFilter::GET,
            Method::Post => MethodFilter::POST,
            Method::Put => MethodFilter::PUT,
            Method::Patch => MethodFilter::PATCH,
            Method::Delete => MethodFilter::DELETE,
            Method::Head => MethodFilter::HEAD,
        }
    }
}

// TODO: impl in lamprey-backend-services or lamprey-backend-core?
/// global state for the server
#[derive(Clone)]
pub struct Globals {
    inner: (),
}

impl Globals {
    pub fn config(&self) -> &Config {
        todo!()
    }
}
