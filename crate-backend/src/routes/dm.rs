use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::{MessageSync, Permission, RelationshipType};
use http::StatusCode;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::{Error, Result};
use crate::routes::util::Auth;
use crate::types::UserIdReq;
use crate::{routes2, ServerState};

// TODO: merge with channel_create_dm
/// Dm initialize
///
/// Get or create a direct message thread.
#[handler(routes::dm_init)]
async fn dm_init(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::dm_init::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();

    srv.perms
        .for_room3(Some(auth.user.id), crate::types::SERVER_ROOM_ID)
        .await?
        .ensure_view()?
        .needs(Permission::DmCreate)
        .check()?;

    // you can't dm webhooks
    let target_user = data.user_get(req.target_id).await?;
    if !target_user.can_dm() {
        return Err(ApiError::from_code(ErrorCode::CannotDmThisUser).into());
    }

    let target_allows_dms = srv
        .perms
        .allows_dm_from_user(auth.user.id, req.target_id)
        .await?;

    if !target_allows_dms {
        let is_friend = data
            .user_relationship_get(auth.user.id, req.target_id)
            .await?
            .is_some_and(|r| r.relation == Some(RelationshipType::Friend));

        if !is_friend {
            return Err(ApiError::from_code(ErrorCode::DmsNotAllowedFromThisUser).into());
        }
    }

    let (thread, is_new) = srv
        .users
        .init_dm(auth.user.id, req.target_id, false)
        .await?;

    s.broadcast(MessageSync::ChannelCreate {
        channel: Box::new(thread.clone()),
    })?;
    if is_new {
        Ok((StatusCode::CREATED, Json(thread)))
    } else {
        Ok((StatusCode::OK, Json(thread)))
    }
}

// TODO: move to channels.rs
/// Dm get
///
/// Get a direct message room.
#[handler(routes::dm_get)]
async fn dm_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::dm_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let Some(thread_id) = data.dm_get(auth.user.id, req.target_id).await? else {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownDm)));
    };
    let srv = s.services();
    let thread = srv.channels.get(thread_id, Some(auth.user.id)).await?;
    Ok(Json(thread))
}

// TODO: move to channels.rs
// TODO: rename to /api/v1/user/@self/channel
/// Dm list
///
/// List direct message channels. Ordered by the last message version id, so
/// recently active dms come first.
#[handler(routes::dm_list)]
async fn dm_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::dm_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth.user.id != target_user_id {
        return Err(Error::MissingPermissions);
    }

    let data = s.data();
    let mut res = data.dm_list(auth.user.id, req.pagination).await?;

    let srv = s.services();
    let mut threads = vec![];
    for t in &res.items {
        threads.push(srv.channels.get(t.id, Some(auth.user.id)).await?);
    }
    res.items = threads;

    Ok(Json(res))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(dm_init))
        .routes(routes2!(dm_get))
        .routes(routes2!(dm_list))
}
