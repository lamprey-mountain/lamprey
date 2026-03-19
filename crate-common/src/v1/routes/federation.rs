use lamprey_macros::endpoint;

/// Server keys get
///
/// Get the signing keys of a server
#[endpoint(
    get,
    path = "/server/{hostname}",
    tags = ["federation"],
    scopes = [Full],
    response(OK, body = ServerKeys, description = "ok"),
)]
pub mod server_keys_get {
    use crate::v1::types::federation::ServerKeys;

    pub struct Request {
        #[path]
        pub hostname: String,
    }

    pub struct Response {
        #[json]
        pub keys: ServerKeys,
    }
}

/// Server user ensure
///
/// Create a user representing a user on the requesting server
#[endpoint(
    post,
    path = "/server/{hostname}/user",
    tags = ["federation"],
    scopes = [Full],
    response(OK, body = ServerUserCreate, description = "ok"),
)]
pub mod server_user_ensure {
    use crate::v1::types::federation::{ServerUserCreate, ServerUserCreateRequest};

    pub struct Request {
        #[path]
        pub hostname: String,

        #[json]
        pub user: ServerUserCreateRequest,
    }

    pub struct Response {
        #[json]
        pub user: ServerUserCreate,
    }
}

/// Server sync handle
///
/// Handle MessageSync events. Used to proxy events to connected clients.
#[endpoint(
    post,
    path = "/server/{hostname}/sync",
    tags = ["federation"],
    scopes = [Full],
    response(ACCEPTED, description = "ok"),
)]
pub mod server_sync_handle {
    pub struct Request {
        #[path]
        pub hostname: String,

        #[json]
        pub sync: Vec<u8>,
    }

    pub struct Response;
}
