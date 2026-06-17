use std::sync::Arc;

use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    middleware,
    response::{Html, IntoResponse},
    routing::get,
};
use common::v1::types::{
    MessageClient, MessageId, MessageSync, PaginationQuery,
    error::{ApiError, ErrorCode},
    misc::ApplicationIdReq,
};
use http::{HeaderName, header};
use lamprey_backend_core::{Error, config::ListenTransport};
use tower_http::{
    catch_panic::CatchPanicLayer, cors::CorsLayer, propagate_header::PropagateHeaderLayer,
    sensitive_headers::SetSensitiveHeadersLayer, trace::TraceLayer,
};
use tracing::warn;
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;

use crate::{
    ServerState,
    routes::{self, util::script_http::script_http},
    serve::utoipa_utils::{BadgeModifier, NestedTags},
    types,
};

#[cfg(feature = "embed-frontend")]
mod frontend;

pub mod server;
mod utoipa_utils;

// NOTE: the `sync` tag doesn't seem to show up, so i moved its docs to index.md
#[derive(OpenApi)]
#[openapi(
    components(schemas(
        types::Room,
        types::RoomPatch,
        types::User,
        types::Channel,
        types::ChannelPatch,
        types::Message,
        types::RoomMember,
        types::Role,
        types::RolePatch,
        // utoipa seems to forget to add these types specifically
        types::UserIdReq,
        ApplicationIdReq,
        types::UserListParams,
        types::UserListFilter,
        MessageSync,
        MessageClient,
        PaginationQuery<MessageId>,
        common::v1::types::pagination::PaginationResponse<types::Message>,
        types::emoji::EmojiCustom,
        types::emoji::EmojiOwner,
        types::reaction::ReactionKey,
        common::v1::types::document::DocumentStateVector,
        common::v1::types::document::DocumentUpdate,
        common::v1::types::document::DocumentBranch,
        common::v1::types::document::DocumentBranchState,
        common::v1::types::document::DocumentBranchListParams,
        common::v1::types::document::DocumentBranchCreate,
        common::v1::types::document::DocumentBranchPatch,
        common::v1::types::document::DocumentBranchMerge,
        common::v1::types::document::DocumentRevisionId,
        common::v1::types::document::DocumentTag,
        common::v1::types::document::DocumentTagCreate,
        common::v1::types::document::DocumentTagPatch,
        common::v1::types::document::HistoryParams,
        common::v1::types::document::Changeset,
        common::v1::types::document::HistoryPagination,
        common::v1::types::document::SerdocPut,
        common::v1::types::document::DocumentPatch,
        common::v1::types::document::Wiki,
        common::v1::types::document::WikiPatch,
        common::v1::types::document::serialized::Serdoc,
        // ack types
        common::v1::types::ack::AckCreate,
        common::v1::types::ack::AckBulk,
        common::v1::types::ack::AckBulkItem,
        // session types
        types::SessionToken,
        // auth types
        common::v1::types::auth::WebauthnAuthenticator,
        common::v1::types::auth::TotpRecoveryCode,
        // reaction types
        common::v1::types::reaction::ReactionListItem,
        // message types
        common::v1::types::message::PinsReorderItem,
        // push types
        common::v1::types::push::PushCreate,
        common::v1::types::push::PushInfo,
        common::v1::types::push::PushCreateKeys,
        // room template types
        common::v1::types::room_template::RoomTemplate,
        common::v1::types::room_template::RoomTemplateCode,
        common::v1::types::room_template::RoomTemplateSnapshot,
        common::v1::types::room_template::RoomTemplateChannel,
        common::v1::types::room_template::RoomTemplateRole,
        // search types
        common::v1::types::search::RoomSearchOrderField,
        common::v1::types::search::MessageSearchOrderField,
        common::v1::types::search::ChannelSearchOrderField,
        common::v1::types::search::MediaSearchOrderField,
        common::v1::types::search::AuditLogSearchOrderField,
        common::v1::types::search::UserSearchOrderField,
        common::v1::types::search::Order,
        // room analytics types
        common::v1::types::room_analytics::Aggregation,
        common::v1::types::room_analytics::AnalyticsInvitesOrigin,
        common::v1::types::room_analytics::AnalyticsChannel,
        common::v1::types::room_analytics::AnalyticsInvites,
        common::v1::types::room_analytics::AnalyticsMembersCount,
        common::v1::types::room_analytics::AnalyticsMembersJoin,
        common::v1::types::room_analytics::AnalyticsMembersLeave,
        common::v1::types::room_analytics::AnalyticsOverview,
        // application/integration types
        common::v1::types::application::Integration,
        // moderation types
        common::v1::types::moderation::ReportReason,
        common::v1::types::moderation::ReportDestination,
        // automod types
        common::v1::types::automod::AutomodRule,
        common::v1::types::automod::AutomodRuleCreate,
        common::v1::types::automod::AutomodTrigger,
        common::v1::types::automod::AutomodAction,
        common::v1::types::automod::AutomodTarget,
        // tag types
        common::v1::types::tag::Tag,
        common::v1::types::tag::TagCreate,
        common::v1::types::tag::TagPatch,
        // server types
        common::v1::types::server::ServerAutomodList,
        common::v1::types::server::ServerMediaScanner,
        // federation types
        common::v1::types::federation::ServerKey,
        // user connection types
        common::v1::types::user_connection::ConnectionMetadata,
        common::v1::types::user_connection::ConnectionValue,
        common::v1::types::user_connection::ConnectionVisibility,
        // user relationship types
        types::Relationship,
        common::v1::types::user::Ignore,
        common::v1::types::user::RelationshipType,
        // room member types
        types::RoomMemberOrigin,
        common::v1::types::room_member::RoomMemberSearchResponse,
        // harvest types
        common::v1::types::harvest::Harvest,
        common::v1::types::harvest::HarvestCreateUser,
        common::v1::types::harvest::HarvestCreateRoom,
        common::v1::types::harvest::HarvestStatus,
        // auth password types
        common::v1::types::auth::PasswordExec,
        common::v1::types::auth::PasswordExecIdent,
        // user search types
        common::v1::types::user::UserSearch,
        common::v1::types::user::UserSearchSortField,
        // relationship types
        common::v1::types::user::RelationshipWithUserId,
        common::v1::types::user::UserWithRelationship,
        // component types
        common::v1::types::components::ComponentId,
        common::v1::types::components::ComponentCustomId,
        common::v1::types::components::ButtonStyle,
        common::v1::types::components::Component<common::v1::types::components::Create>,
        common::v1::types::components::Component<common::v1::types::components::Canonical>,
        common::v1::types::components::Component<common::v1::types::components::Encrypted>,
        common::v1::types::components::ComponentType<common::v1::types::components::Create>,
        common::v1::types::components::ComponentType<common::v1::types::components::Canonical>,
        common::v1::types::components::ComponentType<common::v1::types::components::Encrypted>,
        common::v1::types::components::Components<common::v1::types::components::Create>,
        common::v1::types::components::Components<common::v1::types::components::Canonical>,
        common::v1::types::components::Components<common::v1::types::components::Encrypted>,
        // flume types
        common::v1::types::message::flume::FlumeCreate,
        common::v1::types::message::flume::FlumeDelta,
        common::v1::types::message::flume::FlumeAppend,
        common::v1::types::message::flume::FlumeReplace,
        common::v1::types::message::flume::FlumeState,
        common::v1::types::message::flume::MessageFlume,
        // script types
        common::v1::types::redex::Redex,
        common::v1::types::redex::RedexCreate,
        common::v1::types::redex::RedexContentUpdate,
        common::v1::types::redex::RedexVersion,
        common::v1::types::redex::RedexStatus,
        common::v1::types::redex::RedexFormat,
        common::v1::types::redex::RedexLocation,
        common::v1::types::redex::RedexLocationUpdate,
        common::v1::types::redex::RedexMetadata,
        common::v1::types::redex::RedexHandler,
        common::v1::types::redex::RedexHandlerType,
        common::v1::types::redex::RedexCapability,
        common::v1::types::redex::RedexPermission,
        common::v1::types::redex::RedexPermissionGrant,
        common::v1::types::redex::RedexVersionStatus,
        common::v1::types::redex::Eval,
        common::v1::types::redex::EvalStatus,
        common::v1::types::redex::EvalLogEntry,
        common::v1::types::redex::EvalCreateManual,
        common::v1::types::redex::RedexDependency,
        common::v1::types::redex::RedexDependencyLink,
        common::v1::types::redex::RedexDependencyGraph,
        common::v1::types::redex::RedexDependenciesUpdate,
        common::v1::types::redex::EvalLogLevel,
        common::v1::types::redex::EvalLogSource,
        // media types
        common::v2::types::media::Media,
        common::v2::types::media::MediaReference,
        common::v2::types::media::MediaStatus,
        common::v2::types::media::MediaMetadata,
        common::v2::types::media::MediaScan,
        common::v2::types::media::MediaQuarantine,
        // interactions
        common::v1::types::interactions::InteractionCreate,
        common::v1::types::interactions::InteractionCreateType,
        common::v1::types::interactions::Interaction,
        common::v1::types::interactions::InteractionType,
        common::v1::types::interactions::InteractionResponseCreate,
        common::v1::types::interactions::InteractionResponseCreateType,
        common::v1::types::interactions::InteractionResponse,
        // voice types
        common::v1::types::voice::messages::SignallingEvent,
        common::v1::types::voice::messages::SignallingCommand,
    )),
    modifiers(&BadgeModifier, &NestedTags),
    info(
        title = "api doccery",
        description = include_str!("../../docs/index.md"),
    ),
    tags(
        (name = "sync", description = include_str!("../../docs/sync.md")),
        (name = "auth", description = include_str!("../../docs/auth.md")),
    ),
)]
struct ApiDoc;

/// create an axum router
pub fn create_router(state: Arc<ServerState>) -> Router {
    let (router, mut api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest(
            "/api",
            routes::routes(Arc::clone(&state)).fallback(api_fallback),
        )
        .route("/metrics", get(routes::metrics::get_metrics))
        .route("/.well-known/lamprey-mountain", get(routes::well_known))
        .with_state(state.clone())
        .split_for_parts();

    NestedTags.modify(&mut api);
    BadgeModifier.modify(&mut api);
    let router = router
        .route("/api/docs.json", get(|| async { Json(api) }))
        .route(
            "/api/docs",
            get(|| async { Html(include_str!("../scalar.html")) }),
        );

    #[cfg(not(feature = "embed-frontend"))]
    let router = router.route("/", get(|| async { "it works!" }));
    #[cfg(feature = "embed-frontend")]
    let router = router
        .route(
            "/invite/{code}",
            get(frontend::invite_meta_handler).with_state(state.clone()),
        )
        .fallback_service(axum::routing::get(frontend::frontend_handler).with_state(state.clone()));

    router
        .layer(middleware::from_fn_with_state(state.clone(), script_http))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 16))
        .layer(cors())
        .layer(SetSensitiveHeadersLayer::new([header::AUTHORIZATION]))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            routes::util::audit_log_middleware,
        ))
        .layer(CatchPanicLayer::new())
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            "x-trace-id",
        )))
}

async fn api_fallback() -> impl IntoResponse {
    Error::from(ApiError::from_code(ErrorCode::NotFound))
}

fn cors() -> CorsLayer {
    use header::{AUTHORIZATION, CONTENT_TYPE, HeaderName};
    const UPLOAD_OFFSET: HeaderName = HeaderName::from_static("upload-offset");
    const UPLOAD_LENGTH: HeaderName = HeaderName::from_static("upload-length");
    const IDEMPOTENCY_KEY: HeaderName = HeaderName::from_static("idempotency-key");
    const REASON: HeaderName = HeaderName::from_static("x-reason");
    const PUPPET_ID: HeaderName = HeaderName::from_static("x-puppet-id");
    CorsLayer::very_permissive()
        .expose_headers([CONTENT_TYPE, UPLOAD_OFFSET, UPLOAD_LENGTH])
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            UPLOAD_OFFSET,
            UPLOAD_LENGTH,
            IDEMPOTENCY_KEY,
            REASON,
            PUPPET_ID,
        ])
}

pub async fn serve_transport(
    transport: ListenTransport,
    router: axum::Router,
) -> Result<(), Error> {
    match transport {
        ListenTransport::Tcp { address, port } => {
            let listener = tokio::net::TcpListener::bind((address, port)).await?;
            axum::serve(listener, router).await?;
        }
        ListenTransport::Unix { path } => {
            if let Some(p) = path.parent() {
                tokio::fs::create_dir_all(p).await?;
            }
            if path.exists() {
                warn!("deleting existing socket {}", path.display());
                tokio::fs::remove_file(&path).await?;
            }
            let listener = tokio::net::UnixListener::bind(&path)?;
            let res = axum::serve(listener, router).await;
            let _ = tokio::fs::remove_file(path).await;
            res?;
        }
    }

    Ok(())
}
