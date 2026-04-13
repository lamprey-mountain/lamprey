use lamprey_macros::endpoint;

/// Server keys get
///
/// Get the signing keys of a server
#[endpoint(
    get,
    path = "/server/{hostname}/keys",
    tags = ["federation"],
    scopes = [Full],
    response(OK, body = ServerKeys, description = "ok"),
)]
pub mod server_keys_get {
    use crate::v1::types::{federation::ServerKeys, misc::ServerReq};

    pub struct Request {
        #[path]
        pub hostname: ServerReq,
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
    use crate::v1::types::{
        federation::{ServerUserCreate, ServerUserCreateRequest},
        misc::ServerReq,
    };

    pub struct Request {
        #[path]
        pub hostname: ServerReq,

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
    response(ACCEPTED, body = ServerSyncResponse, description = "ok"),
)]
pub mod server_sync_handle {
    use crate::v1::types::{
        federation::{ServerSyncRequest, ServerSyncResponse},
        misc::ServerReq,
    };

    pub struct Request {
        #[path]
        pub hostname: ServerReq,

        #[json]
        pub sync: ServerSyncRequest,
    }

    pub struct Response {
        #[json]
        pub resp: ServerSyncResponse,
    }
}

/// Server ping
///
/// Check if a server is alive.
#[endpoint(
    post,
    path = "/server/{hostname}/ping",
    tags = ["federation"],
    scopes = [Full],
    response(OK, body = ServerPingResponse, description = "ok"),
)]
pub mod server_ping {
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    use crate::v1::types::misc::ServerReq;

    pub struct Request {
        #[path]
        pub hostname: ServerReq,
    }

    #[derive(Serialize)]
    pub struct Response {
        #[json]
        pub body: PingResponse,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct PingResponse {
        /// always true
        pub ok: bool,
    }
}
