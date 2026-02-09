use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::types::server::{
    ServerAuth, ServerAuthOauth, ServerFeatures, ServerInfo, ServerMedia, ServerModeration,
    ServerRegistration, ServerVersion, ServerVoice, ServerVoiceSfu, ServerWebPush,
};
use common::v1::types::Permission;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::ServerState;

use super::util::Auth;

/// Server information
#[utoipa::path(
    get,
    path = "/server/@self",
    tags = ["server"],
    responses(
        (status = OK, body = ServerInfo, description = "Get server info success"),
    )
)]
async fn server_info(State(s): State<Arc<ServerState>>) -> Result<impl IntoResponse> {
    let data = s.data();
    let config_internal = data.config_get().await?;

    let info = ServerInfo {
        api_url: s.config.api_url.clone(),
        sync_url: s.config.api_url.join("/api/v1/sync").unwrap(),
        html_url: s.config.html_url.clone(),
        cdn_url: s.config.cdn_url.clone(),
        features: ServerFeatures {
            registration: Some(ServerRegistration { enabled: true }),
            auth: Some(ServerAuth {
                supports_totp: true,
                supports_webauthn: false,
                oauth_providers: s
                    .config
                    .oauth_provider
                    .iter()
                    .map(|(id, _)| ServerAuthOauth {
                        id: id.to_owned(),
                        name: id.to_owned(),
                    })
                    .collect(),
            }),
            media: Some(ServerMedia {
                max_file_size: s.config.media_max_size,
            }),
            voice: Some(ServerVoice {}),
            web_push: config_internal.map(|c| ServerWebPush {
                vapid_public_key: c.vapid_public_key,
            }),
        },
        version: ServerVersion {
            implementation: "chat-server".to_string(), // or get from config/env
            version: env!("CARGO_PKG_VERSION").to_string(), // get from cargo
            extra: std::collections::HashMap::new(),   // could add custom metadata
        },
    };
    Ok(Json(info))
}

/// Server moderation capabilities
#[utoipa::path(
    get,
    path = "/server/@self/moderation",
    tags = ["server"],
    responses(
        (status = OK, body = ServerModeration, description = "Get server moderation capabilities success"),
    )
)]
async fn server_moderation(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    // TODO: let server admins configure this
    let moderation = ServerModeration {
        automod_lists: vec![],
        media_scanners: vec![],
    };
    Ok(Json(moderation))
}

/// Server voice sfus
#[utoipa::path(
    get,
    path = "/server/@self/voice",
    tags = ["server", "badge.server-perm.Admin"],
    responses(
        (status = OK, body = Vec<ServerVoiceSfu>, description = "Get server voice sfus success"),
    )
)]
async fn server_voice(auth: Auth, State(s): State<Arc<ServerState>>) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure_server(Permission::Admin)?;

    Ok(Json(vec![] as Vec<ServerVoiceSfu>))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(server_info))
        .routes(routes!(server_moderation))
        .routes(routes!(server_voice))
}
