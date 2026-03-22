use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use common::v1::routes;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::{Error, Result};
use crate::routes::util::Auth;
use crate::{routes2, ServerState};

/// Report create server (TODO)
///
/// Create and send a report to the server operators
#[handler(routes::report_create_server)]
async fn report_create_server(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::report_create_server::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Report create room (TODO)
///
/// Create and send a report to the room admins/moderators
#[handler(routes::report_create_room)]
async fn report_create_room(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::report_create_room::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

// NOTE: do i need routes for listing reports? if reports are special types of threads, maybe not?
// i could have GET /server/report and GET /room/{room_id}/report paginate reports you sent
// paginating all reports for mods is kind of useless considering reports will be threads

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(report_create_server))
        .routes(routes2!(report_create_room))
}
