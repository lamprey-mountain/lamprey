use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::v1::types::application::Scope;
use common::v1::types::misc::InteractionMessageReq;
use common::v1::{routes, types::Permission};
use lamprey_macros::handler;
use tracing::warn;
use utoipa_axum::router::OpenApiRouter;

use crate::routes::util::auth::Auth4;
use crate::{Error, ServerState, routes2};

use super::util::Auth;
use crate::error::Result;

/// Interaction create
#[handler(routes::interaction_create)]
async fn interaction_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::interaction_create::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    // TODO: ensure target application isn't suspended too

    let srv = s.services();

    if let Some(channel_id) = req.create.ty.channel_id() {
        let mut perms = srv
            .perms
            .for_channel3(Some(auth.user.id), channel_id)
            .await?
            .ensure_view()?;
        perms
            .needs_unlocked()
            // .needs_slowmode_message_bypass() // NOTE: should interactions be limited by slowmode?
            .needs(Permission::MessageCreate) // TODO: separate InteractionCreate permission
            .check()?;
    } else {
        warn!("not enforcing any permissions for interaction not in a channel");
    }

    let inter = srv
        .interactions
        .create(auth.user.id, req.idempotency_key, req.create)
        .await?;

    Ok((StatusCode::CREATED, Json(inter)))
}

/// Interaction respond
#[handler(routes::interaction_respond)]
async fn interaction_respond(
    State(s): State<Arc<ServerState>>,
    req: routes::interaction_respond::Request,
) -> Result<impl IntoResponse> {
    let srv = s.services();

    let inter = srv
        .interactions
        .respond(req.interaction_id, req.token, req.response)
        .await?;

    Ok((StatusCode::OK, Json(inter)))
}

/// Interaction message create
#[handler(routes::interaction_message_create)]
async fn interaction_message_create(
    auth: Auth4,
    State(s): State<Arc<ServerState>>,
    req: routes::interaction_message_create::Request,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let inter = srv.interactions.get(req.interaction_id).await?;

    // FIXME: use timing safe equals
    // or better yet, make token a newtype that is timing safe and cant be logged by default
    // also, maybe move this check into the interactions service?
    if inter.inner().token != Some(req.token) {
        // TODO: make this an api ErrorCode
        return Err(Error::BadStatic("unknown or expired interaction"));
    }

    let channel_id = inter
        .inner()
        .ty
        .channel_id()
        .ok_or(Error::BadStatic("interaction type has no channel id"))?;

    let chan = srv.channels.get(channel_id, None).await?;
    chan.ensure_has_text()?; // NOTE: maybe i should make interaction_message_create more flexible? eg. start a comment thread for document interactions?

    // TODO: support timestamp massaging?
    // let header_timestamp = req.timestamp.and_then(|secs| {
    //     time::OffsetDateTime::from_unix_timestamp(secs)
    //         .ok()
    //         .map(Time::from)
    // });

    let message = srv
        .messages
        .create(channel_id, &auth, req.idempotency_key, req.message, None)
        .await?;

    Ok((StatusCode::CREATED, Json(message)))
}

/// Interaction message get
#[handler(routes::interaction_message_get)]
async fn interaction_message_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::interaction_message_get::Request,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let inter = srv.interactions.get(req.interaction_id).await?;

    if inter.inner().token != Some(req.token) {
        return Err(Error::BadStatic("unknown or expired interaction"));
    }

    let channel_id = inter
        .inner()
        .ty
        .channel_id()
        .ok_or(Error::BadStatic("interaction type has no channel id"))?;

    let message_id = match req.message_id {
        InteractionMessageReq::MessageOriginal => todo!(),
        InteractionMessageReq::MessageId(id) => id,
    };

    let message = srv
        .messages
        .get(channel_id, message_id, Some(auth.user.id))
        .await?;
    Ok(Json(message))
}

/// Interaction message edit
#[handler(routes::interaction_message_edit)]
async fn interaction_message_edit(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::interaction_message_edit::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Interaction message delete
#[handler(routes::interaction_message_delete)]
async fn interaction_message_delete(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::interaction_message_delete::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(interaction_create))
        .routes(routes2!(interaction_respond))
        .routes(routes2!(interaction_message_create))
        .routes(routes2!(interaction_message_get))
        .routes(routes2!(interaction_message_edit))
        .routes(routes2!(interaction_message_delete))
}
