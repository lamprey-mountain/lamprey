use common::v1::types::server::{
    ServerAuth, ServerAuthOauth, ServerFeatures, ServerInfo, ServerMedia, ServerModeration,
    ServerRedex, ServerRegistration, ServerVersion, ServerVoice, ServerVoiceHealth,
    ServerVoiceHealthSfu, ServerWebPush,
};
use common::v1::types::{Permission, SERVER_ROOM_ID, oauth::Scope};
use lamprey_backend_core::types::permission::{CheckPermissions, Permissions2};

use crate::prelude::*;

#[handler(routes::server_info)]
pub async fn info(
    req: Req<routes::server_info::Endpoint>,
) -> Result<routes::server_info::Response> {
    let globals = req.globals();
    let config = globals.config();

    let config_internal = globals
        .begin_read()
        .await
        .cast_internal()?
        .config_get()
        .await
        .cast_internal()?;

    let mut extra = std::collections::HashMap::new();
    extra.insert(
        "debug".to_string(),
        option_env!("VERGEN_CARGO_DEBUG")
            .unwrap_or("unknown")
            .to_string(),
    );
    extra.insert(
        "target".to_string(),
        option_env!("VERGEN_CARGO_TARGET_TRIPLE")
            .unwrap_or("unknown")
            .to_string(),
    );
    extra.insert(
        "rev".to_string(),
        option_env!("VERGEN_GIT_SHA")
            .unwrap_or("unknown")
            .to_string(),
    );
    extra.insert(
        "rustc_semver".to_string(),
        option_env!("VERGEN_RUSTC_SEMVER")
            .unwrap_or("unknown")
            .to_string(),
    );
    extra.insert(
        "rustc_llvm".to_string(),
        option_env!("VERGEN_RUSTC_LLVM_VERSION")
            .unwrap_or("unknown")
            .to_string(),
    );
    extra.insert(
        "rustc_rev".to_string(),
        option_env!("VERGEN_RUSTC_COMMIT_HASH")
            .unwrap_or("unknown")
            .to_string(),
    );
    extra.insert(
        "rustc_channel".to_string(),
        option_env!("VERGEN_RUSTC_CHANNEL")
            .unwrap_or("unknown")
            .to_string(),
    );

    let server = ServerInfo {
        api_url: config.api_url.clone(),
        sync_url: config.api_url.join("/api/v1/sync").unwrap(),
        html_url: config.html_url.clone(),
        cdn_url: config.cdn_url.clone(),
        features: ServerFeatures {
            registration: Some(ServerRegistration { enabled: true }),
            auth: Some(ServerAuth {
                supports_totp: true,
                supports_webauthn: true,
                oauth_providers: config
                    .oauth_provider
                    .iter()
                    .map(|(id, _)| ServerAuthOauth {
                        id: id.to_owned(),
                        name: id.to_owned(),
                    })
                    .collect(),
            }),
            media: Some(ServerMedia {
                max_file_size: config.media.max_size,
            }),
            voice: Some(ServerVoice {}),
            web_push: config_internal.map(|c| ServerWebPush {
                vapid_public_key: c.vapid_public_key,
            }),
            #[cfg(feature = "webtransport")]
            webtransport: config.webtransport.as_ref().and_then(|_| {
                let cert = globals.services().config.webtransport_cert();
                let sync_url = config.webtransport_url()?;
                Some(common::v1::types::server::ServerWebtransport {
                    certificate_hashes: vec![cert],
                    sync_url,
                })
            }),
            #[cfg(not(feature = "webtransport"))]
            webtransport: None,
            redex: config.scripts.enabled.then(|| ServerRedex {
                suffix: config.scripts.public_suffix().map(|s| s.to_owned()),
            }),
        },
        version: ServerVersion {
            implementation: "kerosene".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            extra,
        },
    };

    Ok(routes::server_info::Response { server })
}

#[handler(routes::server_moderation)]
pub async fn moderation(
    _req: Req<routes::server_moderation::Endpoint>,
) -> Result<routes::server_moderation::Response> {
    let moderation = ServerModeration {
        automod_lists: vec![],
        media_scanners: vec![],
    };

    Ok(routes::server_moderation::Response { moderation })
}

#[handler(routes::server_voice)]
pub async fn voice(
    req: Req<routes::server_voice::Endpoint>,
) -> Result<routes::server_voice::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let mut perms: Permissions2<CheckPermissions> = srv
        .perms
        .for_room3(Some(user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::Admin);
    perms.check()?;

    let mut sfus = vec![];
    for sfu in srv.voice.sfus.iter() {
        let stats = sfu.stats.read().await;
        sfus.push(ServerVoiceHealthSfu {
            id: sfu.id,
            connected_at: sfu.connected_at,
            bandwidth_usage: stats.bandwidth_usage,
            bandwidth_max: stats.bandwidth_max,
            count_peer: stats.peer_count,

            // FIXME: populate
            count_users: 0,
            count_tracks: 0,
        });
    }

    Ok(routes::server_voice::Response {
        body: ServerVoiceHealth { sfus },
    })
}

export_routes!(info, moderation, voice);
