//! rest routes

use axum::{Json, extract::State, response::IntoResponse};
use common::v1::types::federation::WellKnown;
use kerosene_services::globals::server_state::ServerState;
use utoipa_axum::router::OpenApiRouter;

use crate::prelude::*;

mod admin;
mod application;
mod auth;
mod automod;
mod calendar;
mod debug;
mod dm;
mod document;
#[cfg(feature = "e2ee")]
mod e2ee;
mod emoji;
mod federation;
mod flume;
mod interaction;
mod internal;
mod invite;
mod media;
mod moderation;
mod notification;
mod oauth;
mod pack;
mod preferences;
mod public;
mod push;
mod reaction;
mod redex;
mod room_analytics;
mod room_member;
mod room_template;
mod search;
mod server;
mod session;
mod sync;
mod tag;
mod user_connection;
mod user_email;
mod voice;
mod webhook;

pub mod metrics;
pub mod util;

fn routes_v1(s: Arc<ServerState>) -> OpenApiRouter<Arc<ServerState>> {
    let router = OpenApiRouter::new()
        .merge(admin::routes())
        .merge(application::routes())
        .merge(auth::routes())
        .merge(automod::routes())
        .merge(calendar::routes())
        .merge(debug::routes())
        .merge(dm::routes())
        .merge(document::routes())
        .merge(emoji::routes())
        .merge(federation::routes(Arc::clone(&s)))
        .merge(flume::routes())
        .merge(interaction::routes())
        .merge(internal::routes())
        .merge(invite::routes())
        .merge(media::routes())
        .merge(moderation::routes())
        .merge(notification::routes())
        .merge(oauth::routes())
        .merge(pack::routes())
        .merge(public::routes())
        .merge(push::routes())
        .merge(reaction::routes())
        .merge(room_analytics::routes())
        .merge(room_member::routes())
        .merge(room_template::routes())
        .merge(redex::routes())
        .merge(search::routes())
        .merge(server::routes())
        .merge(session::routes())
        .merge(sync::routes())
        .merge(tag::routes())
        .merge(preferences::routes())
        .merge(user_connection::routes())
        .merge(user_email::routes())
        .merge(voice::routes())
        .merge(webhook::routes());

    #[cfg(feature = "e2ee")]
    {
        router = router.merge(e2ee::routes());
    }

    router
}

/// Get well known
pub async fn well_known(State(s): State<Globals>) -> Result<impl IntoResponse> {
    Ok(Json(WellKnown {
        api_url: s.config().api_url.clone(),
        cdn_url: s.config().cdn_url.clone(),
    }))
}

fn routes_v2(_s: Arc<ServerState>) -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
}

pub fn routes(s: Arc<ServerState>) -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .nest("/v1", routes_v1(Arc::clone(&s)))
        .nest("/v2", routes_v2(s))
}
