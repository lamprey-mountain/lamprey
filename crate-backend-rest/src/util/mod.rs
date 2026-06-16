pub mod auth;
pub mod request;
pub mod routes;

pub use auth::Auth;
pub use request::Req;
pub use routes::Routes;

/// global state for the server
#[derive(Clone)]
pub struct Globals {
    inner: (),
}
