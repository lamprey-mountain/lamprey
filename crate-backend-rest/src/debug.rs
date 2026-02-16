use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{extract::State, routing::{get, post}, Json, Router};
use common::v1::types::{ChannelId, EmbedRequest, Permission, RoomId, UserId};
use serde::{Deserialize, Serialize};
use url::Url;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

use crate::ServerState;
use lamprey_backend_core::prelude::*;

#[derive(Debug, Serialize, ToSchema)]
struct ServerInfo {
    version: ServerVersion,
    features: ServerFeatures,

    api_url: Url,
    html_url: Url,
    cdn_url: Url,
}

/// shows some parts of config
#[derive(Debug, Serialize, ToSchema)]
struct ServerFeatures {
    registration: Option<features::Registration>,
    catptcha: Option<features::Captcha>,
    email: Option<features::Email>,
    voice: Option<features::Voice>,
    media: Option<features::Media>,
    url_embed: Option<features::UrlEmbed>,
    oauth: Option<features::Oauth>,
    experiments: Option<features::Experiments>,
}

mod features {
    use serde::Serialize;
    use utoipa::ToSchema;

    #[derive(Debug, Serialize, ToSchema)]
    pub struct Registration {
        // doesn't exist yet
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct Captcha {
        // doesn't exist yet. what captcha providers do i want to support?
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct Email {
        // doesn't exist yet
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct Voice {
        // doesn't exist yet
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct Media {
        pub max_size: u64,
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct UrlEmbed {
        // doesn't exist yet
    }

    /// log in with xyz
    #[derive(Debug, Serialize, ToSchema)]
    pub struct Oauth {
        pub providers: Vec<OauthProvider>,
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct OauthProvider {
        /// friendly name
        pub name: String,

        /// api name
        pub id: String,
    }

    // currently no experiments
    #[derive(Debug, Serialize, ToSchema)]
    pub struct Experiments {
        // "search_message": {},
        // "member_list": {},
        // "inbox": {},
        // "user_friend": {},
        // "user_block": {},
        // "user_ignore": {},
        // inner: HashMap<String, Value>,
    }
}

#[derive(Debug, Serialize, ToSchema)]
struct ServerVersion {
    debug: bool,
    target: &'static str,
    rev: &'static str,
    rustc_semver: &'static str,
    rustc_llvm: &'static str,
    rustc_rev: &'static str,
    rustc_channel: &'static str,
}

#[derive(Debug, Deserialize, ToSchema)]
struct TestPermissionsRequest {
    room_id: RoomId,
    channel_id: Option<ChannelId>,
    user_id: UserId,
}

#[derive(Debug, Serialize, ToSchema)]
struct TestPermissionsResponse {
    permissions: Vec<Permission>,
}

/// Get server info
///
/// in the future, this will become a stable route
#[utoipa::path(
    get,
    path = "/debug/info",
    tag = "debug",
    responses((status = 200, body = ServerInfo, description = "success")),
)]
pub async fn debug_info(State(s): State<Arc<ServerState>>) -> Result<impl IntoResponse> {
    Ok(Json(ServerInfo {
        version: ServerVersion {
            debug: env!("VERGEN_CARGO_DEBUG") == "true",
            target: env!("VERGEN_CARGO_TARGET_TRIPLE"),
            rev: env!("VERGEN_GIT_SHA"),
            rustc_semver: env!("VERGEN_RUSTC_SEMVER"),
            rustc_llvm: env!("VERGEN_RUSTC_LLVM_VERSION"),
            rustc_rev: env!("VERGEN_RUSTC_COMMIT_HASH"),
            rustc_channel: env!("VERGEN_RUSTC_CHANNEL"),
        },
        features: ServerFeatures {
            registration: Some(features::Registration {}),
            catptcha: None,
            email: Some(features::Email {}),
            voice: None, // not advertised for now, too buggy unfortunately
            media: Some(features::Media {
                max_size: s.config.media_max_size,
            }),
            oauth: Some(features::Oauth {
                providers: s
                    .config
                    .oauth_provider
                    .iter()
                    .map(|(id, _)| features::OauthProvider {
                        id: id.to_owned(),
                        name: id.to_owned(),
                    })
                    .collect(),
            }),
            url_embed: Some(features::UrlEmbed {}),
            experiments: None,
        },
        api_url: s.config.api_url.clone(),
        cdn_url: s.config.cdn_url.clone(),
        html_url: s.config.html_url.clone(),
    }))
}

/// Get server version
#[utoipa::path(
    get,
    path = "/debug/version",
    tag = "debug",
    responses(
        (status = 200, body = ServerVersion, description = "success"),
    )
)]
pub async fn debug_version() -> Result<impl IntoResponse> {
    Ok(Json(ServerVersion {
        debug: env!("VERGEN_CARGO_DEBUG") == "true",
        target: env!("VERGEN_CARGO_TARGET_TRIPLE"),
        rev: env!("VERGEN_GIT_SHA"),
        rustc_semver: env!("VERGEN_RUSTC_SEMVER"),
        rustc_llvm: env!("VERGEN_RUSTC_LLVM_VERSION"),
        rustc_rev: env!("VERGEN_RUSTC_COMMIT_HASH"),
        rustc_channel: env!("VERGEN_RUSTC_CHANNEL"),
    }))
}

/// Embed a url
#[utoipa::path(
    post,
    path = "/debug/embed-url",
    tag = "debug",
    responses(
        (status = 202, description = "success"),
    )
)]
pub async fn debug_embed_url(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<EmbedRequest>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let mut embed = ServiceEmbed::generate_inner(&s.inner, auth.user.id, json.url).await?;
    if let Some(m) = &mut embed.media {
        s.presign(m).await?;
    }
    if let Some(m) = &mut embed.thumbnail {
        s.presign(m).await?;
    }
    if let Some(m) = &mut embed.author_avatar {
        s.presign(m).await?;
    }
    Ok(Json(embed))
}

/// Trigger a panic
#[utoipa::path(
    get,
    path = "/debug/panic",
    tag = "debug",
    responses((status = 500, description = "success")),
)]
pub async fn debug_panic() {
    panic!("whoops!")
}

/// Test permissions
///
/// Get the resolved set of permissions for a user
#[utoipa::path(
    post,
    path = "/debug/test-permissions",
    tag = "debug",
    responses((status = 200, body = TestPermissionsResponse, description = "success")),
)]
pub async fn debug_test_permissions(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<TestPermissionsRequest>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    // check that the user has permissions for the room
    let _ = s
        .services()
        .perms
        .for_room(auth.user.id, json.room_id)
        .await?;

    let permissions = if let Some(channel_id) = json.channel_id {
        s.services()
            .perms
            .for_channel(json.user_id, channel_id)
            .await?
    } else {
        s.services()
            .perms
            .for_room(json.user_id, json.room_id)
            .await?
    };

    let mut permissions_vec: Vec<Permission> = permissions.into_iter().collect();
    permissions_vec.sort();

    let response = TestPermissionsResponse {
        permissions: permissions_vec,
    };

    Ok(Json(response))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct CheckHealthResponse {
    ok: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct CheckReadyResponse {
    ok: bool,

    /// is postgres reachable?
    postgres_ok: bool,

    /// is s3 reachable?
    bucket_ok: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct CheckDoctorResponse {
    ok: bool,
    issues: Vec<DoctorIssue>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct DoctorIssue {
    /// whats the name of this check
    name: String,

    /// how bad is it
    severity: DoctorSeverity,

    /// what's wrong
    message: String,

    /// why its a problem
    detail: Option<String>,

    /// how to fix it
    suggestion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
enum DoctorSeverity {
    /// not a problem but still worth knowing
    Info,

    /// this is something you should fix when you have time
    Warning,

    /// this is something you should fix NOW
    Critical,
}

/// Check health
///
/// is this server alive?
#[utoipa::path(
    get,
    path = "/health",
    tag = "debug",
    responses(
        (status = 200, description = "server is healthy"),
    )
)]
pub async fn debug_health() -> Result<impl IntoResponse> {
    Ok(Json(CheckHealthResponse { ok: true }))
}

/// Check ready
///
/// is this server ready to accept requests?
#[utoipa::path(
    get,
    path = "/ready",
    tag = "debug",
    responses(
        (status = 200, description = "server is ready"),
    )
)]
pub async fn debug_ready(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure(Permission::Admin)?;

    Ok(Json(CheckReadyResponse {
        ok: true,
        postgres_ok: true,
        bucket_ok: true,
    }))
}

/// Check doctor
///
/// what's wrong with this server and how do i fix it?
#[utoipa::path(
    get,
    path = "/doctor",
    tag = "debug",
    responses(
        (status = 200, description = "diagnostic information"),
    )
)]
pub async fn debug_doctor(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure(Permission::Admin)?;

    Ok(Json(CheckDoctorResponse {
        ok: true,
        issues: vec![],
    }))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .route("/debug/info", get(debug_info))
        .route("/debug/version", get(debug_version))
        .route("/debug/embed-url", post(debug_embed_url))
        .route("/debug/panic", get(debug_panic))
        .route("/debug/test-permissions", post(debug_test_permissions))
        .route("/health", get(debug_health))
        .route("/ready", get(debug_ready))
        .route("/doctor", get(debug_doctor))
}
