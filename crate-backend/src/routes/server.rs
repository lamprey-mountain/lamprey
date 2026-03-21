use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::server::{
    ServerAuth, ServerAuthOauth, ServerFeatures, ServerInfo, ServerMedia, ServerModeration,
    ServerRegistration, ServerVersion, ServerVoice, ServerVoiceSfu, ServerWebPush,
};
use common::v1::types::Permission;
use lamprey_macros::handler;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{routes2, ServerState};

use super::util::Auth;
use crate::error::Result;

/// Server information
#[handler(routes::server_info)]
async fn server_info(
    State(s): State<Arc<ServerState>>,
    _req: routes::server_info::Request,
) -> Result<impl IntoResponse> {
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
            implementation: "chat-server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            extra: std::collections::HashMap::new(),
        },
    };
    Ok(Json(info))
}

/// Server moderation capabilities
#[handler(routes::server_moderation)]
async fn server_moderation(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::server_moderation::Request,
) -> Result<impl IntoResponse> {
    let moderation = ServerModeration {
        automod_lists: vec![],
        media_scanners: vec![],
    };
    Ok(Json(moderation))
}

/// Server voice sfus
#[handler(routes::server_voice)]
async fn server_voice(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::server_voice::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure_server(Permission::Admin)?;

    Ok(Json(vec![] as Vec<ServerVoiceSfu>))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(server_info))
        .routes(routes2!(server_moderation))
        .routes(routes2!(server_voice))
}
