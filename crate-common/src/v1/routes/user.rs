use lamprey_macros::endpoint;

/// User get
///
/// Get another user, including your relationship
#[endpoint(
    get,
    path = "/user/{user_id}",
    tags = ["user"],
    scopes = [Identify],
    response(OK, body = UserWithRelationship, description = "success"),
    errors(UnknownUser),
)]
pub mod user_get {
    use crate::v1::types::{misc::UserIdReq, UserWithRelationship};

    pub struct Request {
        /// the user id to fetch
        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {
        #[json]
        pub user: UserWithRelationship,
    }
}
