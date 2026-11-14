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
mod federation;
mod flume;
mod internal;
mod invite;
mod moderation;
mod oauth;
mod preferences;
mod public;
mod push;
mod redex;
mod sync;
mod user_connection;
mod user_email;
mod voice;

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
        .merge(federation::routes(Arc::clone(&s)))
        .merge(flume::routes())
        .merge(internal::routes())
        .merge(invite::routes())
        .merge(moderation::routes())
        .merge(oauth::routes())
        .merge(public::routes())
        .merge(push::routes())
        .merge(redex::routes())
        .merge(sync::routes())
        .merge(preferences::routes())
        .merge(user_connection::routes())
        .merge(user_email::routes())
        .merge(voice::routes());

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
