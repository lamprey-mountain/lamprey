use lamprey_macros::endpoint;

/// Push register
///
/// Register web push for this session
#[endpoint(
    post,
    path = "/push",
    tags = ["push"],
    scopes = [Full],
    response(OK, body = PushInfo, description = "ok"),
)]
pub mod push_register {
    use crate::v1::types::push::{PushCreate, PushInfo};

    pub struct Request {
        #[json]
        pub push: PushCreate,
    }

    pub struct Response {
        #[json]
        pub push: PushInfo,
    }
}

/// Push delete
///
/// Remove web push for this session
#[endpoint(
    delete,
    path = "/push",
    tags = ["push"],
    scopes = [Full],
    response(NO_CONTENT, description = "ok"),
)]
pub mod push_delete {
    pub struct Request {}
    pub struct Response {}
}

/// Push get
///
/// Get web push subscription for this session
#[endpoint(
    get,
    path = "/push",
    tags = ["push"],
    scopes = [Full],
    response(OK, body = PushInfo, description = "ok"),
)]
pub mod push_get {
    use crate::v1::types::push::PushInfo;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub push: PushInfo,
    }
}
