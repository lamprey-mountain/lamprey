use lamprey_macros::endpoint;

/// Unfurler debug
#[endpoint(
    post,
    path = "/unfurler/debug",
    tags = ["unfurl"],
    scopes = [Full],
    response(OK, body = UnfurlerDebugResponse, description = "Success"),
)]
pub mod unfurler_debug {
    use crate::v1::types::unfurl::{UnfurlerDebugRequest, UnfurlerDebugResponse};

    pub struct Request {
        #[json]
        pub body: UnfurlerDebugRequest,
    }

    pub struct Response {
        #[json]
        pub body: UnfurlerDebugResponse,
    }
}

/// Unfurler unfurl
#[endpoint(
    post,
    path = "/unfurler/unfurl",
    tags = ["unfurl"],
    scopes = [Full],
    response(OK, body = UnfurlerResponse, description = "Success"),
)]
pub mod unfurler_unfurl {
    use crate::v1::types::unfurl::{UnfurlerRequest, UnfurlerResponse};

    pub struct Request {
        #[json]
        pub body: UnfurlerRequest,
    }

    pub struct Response {
        #[json]
        pub body: UnfurlerResponse,
    }
}
