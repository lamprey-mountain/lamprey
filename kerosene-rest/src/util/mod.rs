use axum::routing::MethodFilter;
use common::util::routes::Method;

pub mod auth;
pub mod headers;
pub mod request;
pub mod routes;

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

pub use lamprey_backend_services::globals::Globals;

macro_rules! export_routes {
    ($($route:ident),* $(,)?) => {
        pub fn register(r: &mut crate::Routes) {
            $($route::register(r);)*
        }
    };
}

pub(crate) use export_routes;
