use std::sync::Arc;
use lamprey_macros::{endpoint, handler};
use axum::extract::State;
async fn __user_get_inner(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: user_get::Request,
) -> Result<impl IntoResponse> {
    ::core::panicking::panic("not yet implemented")
}
async fn user_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    __raw_req: ::axum::extract::Request,
) -> Result<impl ::axum::response::IntoResponse, ::axum::response::Response> {
    use ::axum::response::IntoResponse as _;
    let req = user_get::__extract(__raw_req).await?;
    __user_get_inner(auth, _, req).await.map_err(|e| e.into_response())
}
