use lamprey_macros::{endpoint, endpoint_new};

/// Get well known
#[endpoint_new(
    get,
    path = "/.well-known/lamprey-mountain",
    tags = ["federation"],
    response(OK, body = WellKnown),
)]
pub mod well_known {
    use crate::v1::types::federation::WellKnown;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub info: WellKnown,
    }
}

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

        #[header]
        pub idempotency_key: Option<String>,

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
    use crate::v1::types::{federation::ServerPingResponse, misc::ServerReq};

    pub struct Request {
        #[path]
        pub hostname: ServerReq,
    }

    pub struct Response {
        #[json]
        pub body: ServerPingResponse,
    }
}

/// Server connect
///
/// Start receiving sync events from a remote server.
///
/// Servers should send attempt to reconnect on startup, since remote servers may stop sending events to unreachable servers.
#[endpoint(
    post,
    path = "/server/{hostname}/connect",
    tags = ["federation"],
    scopes = [Full],
    response(OK, body = ServerConnectResponse, description = "connected"),
)]
pub mod server_connect {
    use crate::v1::types::{federation::ServerConnectResponse, misc::ServerReq};

    pub struct Request {
        #[path]
        pub hostname: ServerReq,
    }

    pub struct Response {
        #[json]
        pub body: ServerConnectResponse,
    }
}
