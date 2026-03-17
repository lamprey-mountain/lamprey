use std::sync::Arc;
use lamprey_macros::{endpoint, handler};
use axum::extract::State;

/// User get
///
/// Get another user, including your relationship
#[endpoint(
    get,
    path = "/user/{user_id}",
    tags = ["user"],
    scopes = ["identify"],
    response(status = OK, body = UserWithRelationship, description = "success"),
)]
pub mod user_get {
    pub struct Request {
        #[path]
        pub user_id: UserIdReq,
    }
    pub struct Response {
        #[json]
        pub user: UserWithRelationship,
    }
}

#[handler(user_get)]
async fn user_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: user_get::Request,
) -> Result<impl IntoResponse> {
    todo!()
}
