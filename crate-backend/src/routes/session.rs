use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::SessionIdReq;
use crate::ServerState;

use crate::error::{Error, Result};
use super::util::Auth;

// /// Session create
// #[utoipa::path(
//     post,
//     path = "/session",
//     tags = ["session"],
//     responses(
//         (status = CREATED, description = "success"),
//     )
// )]
// pub async fn session_create(
//     State(s): State<ServerState>,
//     Json(body): Json<SessionCreate>,
// ) -> Result<impl IntoResponse> {
//     let data = s.data();
//     let session = data.session_create(body.user_id, body.name).await?;
//     Ok((StatusCode::CREATED, Json(session)))
// }

// /// Session list
// #[utoipa::path(
//     get,
//     path = "/session",
//     tags = ["session"],
//     responses(
//         (status = OK, description = "success"),
//     )
// )]
// pub async fn session_list(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<()>> {
//     todo!()
// }

// /// Session update
// #[utoipa::path(
//     patch,
//     path = "/session/{session_id}",
//     params(
//         ("session_id", description = "Session id"),
//     ),
//     tags = ["session"],
//     responses(
//         (status = CREATED, description = "success"),
//     )
// )]
// pub async fn session_update(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<()>> {
//     todo!()
// }

/// Session delete
#[utoipa::path(
    delete,
    path = "/session/{session_id}",
    params(
        ("session_id", description = "Session id"),
    ),
    tags = ["session"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
pub async fn session_delete(
    Path(session_id): Path<SessionIdReq>,
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<impl IntoResponse> {
    let session_id = match session_id {
        SessionIdReq::SessionSelf => session.id,
        SessionIdReq::SessionId(session_id) => session_id,
    };
	// if (c.get("session_status") === SessionStatus.Unauthorized && session_id !== c.get("session_id")) {
	// 	return new Response(null, { status: 204 });
	// }
	let data = s.data();
	let target_session = data.session_get(session_id).await?;
    if target_session.user_id != session.user_id {
        return Err(Error::NotFound);
    }
    // TODO: should i restrict deleting other sessions to sudo mode?
    data.session_delete(session_id).await?;
    s.sushi.send(types::MessageServer::DeleteSession { id: session_id })?;
    Ok(StatusCode::NO_CONTENT)
}

/// Session get
#[utoipa::path(
    get,
    path = "/session/{session_id}",
    params(
        ("session_id", description = "Session id"),
    ),
    tags = ["session"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn session_get(
    Path(session_id): Path<SessionIdReq>,
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<impl IntoResponse> {
    let session_id = match session_id {
        SessionIdReq::SessionSelf => session.id,
        SessionIdReq::SessionId(session_id) => session_id,
    };
	// if (c.get("session_status") === SessionStatus.Unauthorized && session_id !== c.get("session_id")) {
	// 	return c.json({ error: "not found" }, 404);
	// }
	let data = s.data();
	let session = data.session_get(session_id).await?;
	Ok(Json(session))
}

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        // .routes(routes!(session_create))
        // .routes(routes!(session_list))
        // .routes(routes!(session_update))
        .routes(routes!(session_get))
        .routes(routes!(session_delete))
}
