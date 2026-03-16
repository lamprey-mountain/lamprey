use lamprey_macros::endpoint;
use common::v1::types::{UserWithRelationship, UserIdReq};

/// User get
///
/// Get another user, including your relationship
#[endpoint(
    get,
    path = "/user/{user_id}",
    tags = ["user"],
    scopes = ["identify"],
    response(status = OK, body = UserWithRelationship, description = "success"),
    errors(UnknownUser),
)]
pub mod user_get {
    pub struct Request {
        /// the user id
        #[path]
        pub user_id: UserIdReq,
    }
    pub struct Response {
        #[json]
        pub user: UserWithRelationship,
    }
}
