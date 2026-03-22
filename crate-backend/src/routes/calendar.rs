// Calendar routes - stub migration
// Note: Most endpoints return Unimplemented due to missing data layer methods

use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use common::v1::routes;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::{Error, Result};
use crate::routes::util::Auth;
use crate::{routes2, ServerState};

/// Calendar event list user (TODO)
#[handler(routes::calendar_event_list_user)]
async fn calendar_event_list_user(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::calendar_event_list_user::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event list
#[handler(routes::calendar_event_list)]
async fn calendar_event_list(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::calendar_event_list::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event create
#[handler(routes::calendar_event_create)]
async fn calendar_event_create(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::calendar_event_create::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event get
#[handler(routes::calendar_event_get)]
async fn calendar_event_get(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::calendar_event_get::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event update
#[handler(routes::calendar_event_update)]
async fn calendar_event_update(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::calendar_event_update::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event delete
#[handler(routes::calendar_event_delete)]
async fn calendar_event_delete(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::calendar_event_delete::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event participant list
#[handler(routes::calendar_event_participant_list)]
async fn calendar_event_participant_list(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::calendar_event_participant_list::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event participant add
#[handler(routes::calendar_event_participant_add)]
async fn calendar_event_participant_add(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::calendar_event_participant_add::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event participant remove
#[handler(routes::calendar_event_participant_remove)]
async fn calendar_event_participant_remove(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::calendar_event_participant_remove::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar overwrite list
#[handler(routes::calendar_overwrite_list)]
async fn calendar_overwrite_list(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::calendar_overwrite_list::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar overwrite put
#[handler(routes::calendar_overwrite_put)]
async fn calendar_overwrite_put(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::calendar_overwrite_put::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(calendar_event_list_user))
        .routes(routes2!(calendar_event_list))
        .routes(routes2!(calendar_event_create))
        .routes(routes2!(calendar_event_get))
        .routes(routes2!(calendar_event_update))
        .routes(routes2!(calendar_event_delete))
        .routes(routes2!(calendar_event_participant_list))
        .routes(routes2!(calendar_event_participant_add))
        .routes(routes2!(calendar_event_participant_remove))
        .routes(routes2!(calendar_overwrite_list))
        .routes(routes2!(calendar_overwrite_put))
}
