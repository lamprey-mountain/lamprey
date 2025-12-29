use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::{ChannelId, EmbedRequest, Permission, RoomId, UserId};
use serde::{Deserialize, Serialize};
use url::Url;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::services::embed::ServiceEmbed;
use crate::ServerState;

use super::util::Auth2;
use crate::error::Result;

#[derive(Debug, Serialize, ToSchema)]
struct ServerInfo {
    version: ServerVersion,
    // supported_api_versions: Vec<String>, // "v1", "v2", etc
    features: ServerFeatures,

    api_url: Url,
    html_url: Url,
    cdn_url: Url,
    // other stuff
    // vapid_key: Bytes, // web push api
    // registration: Registration,

    // // maybe make this status/presence on a special server user
    // motd: Option<String>,

    // // public stats?
    // members_offline: u64,
    // members_online: u64, // make sure not to leak invis/offline users
    // copy room stats here (threads, active threads, messages)
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
        // /// permissions that guest accounts have. if None, guests are readonly.
        // /// guests have a baseline set of permissions, even if this is an empty
        // /// vec
        // pub guests_writable: Option<Vec<GuestPermission>>,

        // /// maximum size of media guests can upload
        // pub guest_media_max_size: u64,

        // /// mime types of files that guest users can upload, empty to disable all file types
        // pub guest_media_allowed_types: Vec<String>,

        // /// whether a server invite is required to join this server (DISABLING NOT RECOMMENDED)
        // pub invite_required: bool,
    }

    // #[derive(Debug, Serialize, ToSchema)]
    // pub enum GuestPermission {
    //     /// can create rooms
    //     /// this is a permission because guests could create a room then lose access to their account
    //     CreateRooms,

    //     /// can speak in voice channels
    //     /// moderating this seems like it could be very painful
    //     Voice,

    //     /// can start direct messages
    //     /// enabling this could be pretty spammy
    //     StartDms,
    // }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct Captcha {
        // doesn't exist yet. what captcha providers do i want to support?
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct Email {
        // /// this supports email-to-thread (if it ever gets implemented?)
        // pub ingest: bool,

        // /// the email address all system emails will come from
        // pub postmaster: String,
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct Voice {
        // /// can allocate voice servers on demand
        // pub dynamic_servers: bool,
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct Media {
        pub max_size: u64,
        // pub allowed_mime_types: Vec<String>,
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct UrlEmbed {
        // /// only will generate url embeds for these sites
        // pub allowed_sites: Vec<String>,
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
        // /// icon for this provider
        // pub icon: MediaId,
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
    tags = ["debug"],
    responses((status = OK, body = ServerInfo, description = "success")),
)]
async fn debug_info(State(s): State<Arc<ServerState>>) -> Result<impl IntoResponse> {
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
    tags = ["debug"],
    responses(
        (status = OK, body = ServerVersion, description = "success"),
    )
)]
async fn debug_version() -> Result<impl IntoResponse> {
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
    tags = ["debug"],
    responses(
        (status = ACCEPTED, description = "success"),
    )
)]
async fn debug_embed_url(
    auth: Auth2,
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
    tags = ["debug"],
    responses((status = INTERNAL_SERVER_ERROR, description = "success")),
)]
async fn debug_panic() {
    panic!("whoops!")
}

/// Test permissions
///
/// Get the resolved set of permissions for a user
#[utoipa::path(
    post,
    path = "/debug/test-permissions",
    tags = ["debug"],
    responses((status = OK, body = TestPermissionsResponse, description = "success")),
)]
async fn debug_test_permissions(
    auth: Auth2,
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

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(debug_info))
        .routes(routes!(debug_version))
        .routes(routes!(debug_embed_url))
        .routes(routes!(debug_panic))
        .routes(routes!(debug_test_permissions))
}
