use std::{str::FromStr, sync::Arc, time::Duration};

use axum::{extract::DefaultBodyLimit, response::Html, routing::get, Json};
use clap::Parser;
use common::v1::types::{misc::ApplicationIdReq, util::Time, AuditLogEntry, AuditLogEntryType};
use figment::providers::{Env, Format, Toml};
use http::{header, HeaderName};
use opendal::layers::LoggingLayer;
use opentelemetry_otlp::WithExportConfig;
use sqlx::postgres::PgPoolOptions;
use tokio::task::JoinSet;
use tower_http::{
    catch_panic::CatchPanicLayer, propagate_header::PropagateHeaderLayer,
    sensitive_headers::SetSensitiveHeadersLayer, trace::TraceLayer,
};
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;

use crate::config::{ListenComponent, ListenTransport};

use lamprey_backend::{
    cli, config, error,
    routes::{self},
    types::{
        self, AuditLogEntryId, DbRoomCreate, DbUserCreate, MessageId, MessageSync, PaginationQuery,
        RoomCreate, RoomMemberPut, RoomType, SERVER_ROOM_ID, SERVER_USER_ID,
    },
    Error, ServerState,
};

use config::Config;
use error::Result;

use crate::util::{cors, BadgeModifier, NestedTags};

mod util;

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
        // utoipa seems to forget to add these types specifically
        types::UserIdReq,
        ApplicationIdReq,
        types::UserListParams,
        types::UserListFilter,
        MessageSync,
        PaginationQuery<MessageId>,
        types::emoji::Emoji,
        types::emoji::EmojiCustom,
        types::emoji::EmojiOwner,
        types::reaction::ReactionKey,
        types::notifications::InboxChannelsOrder,
    )),
    info(
        title = "api doccery",
        description = include_str!("../docs/index.md"),
    ),
    tags(
        (name = "sync", description = include_str!("../docs/sync.md")),
        (name = "auth", description = include_str!("../docs/auth.md")),
    ),
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    let args = cli::Args::parse();

    let config: Config = figment::Figment::new()
        .merge(Toml::file(args.config))
        // .merge(Toml::file("config.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    if let Some(endpoint) = &config.otel_trace_endpoint {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()?;
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .build();
        use opentelemetry::trace::TracerProvider;
        let tracer = provider.tracer("bridge-discord");
        opentelemetry::global::set_tracer_provider(provider);
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default()
            .with(EnvFilter::from_str(&config.rust_log)?)
            .with(tracing_subscriber::fmt::layer())
            .with(telemetry_layer);
        tracing::subscriber::set_global_default(subscriber)?;
    } else {
        let subscriber = Registry::default()
            .with(EnvFilter::from_str(&config.rust_log)?)
            .with(tracing_subscriber::fmt::layer());
        tracing::subscriber::set_global_default(subscriber)?;
    }

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let blobs_builder = opendal::services::S3::default()
        .bucket(&config.s3.bucket)
        .endpoint(config.s3.endpoint.as_str())
        .region(&config.s3.region)
        .access_key_id(&config.s3.access_key_id)
        .secret_access_key(&config.s3.secret_access_key);
    let blobs = opendal::Operator::new(blobs_builder)?
        .layer(LoggingLayer::default())
        .finish();
    blobs.check().await?;

    let state = Arc::new(ServerState::new(config, pool, blobs));

    let srv = state.services();
    let data = state.data();
    if data.user_get(SERVER_USER_ID).await.is_err() {
        data.user_create(DbUserCreate {
            id: Some(SERVER_USER_ID),
            parent_id: None,
            name: "root".to_string(),
            description: None,
            puppet: None,
            registered_at: Some(Time::now_utc()),
            system: true,
        })
        .await?;
    }
    if data.room_get(SERVER_ROOM_ID).await.is_err() {
        srv.rooms
            .create(
                RoomCreate {
                    name: "server".to_string(),
                    description: None,
                    icon: None,
                    public: Some(false),
                },
                SERVER_USER_ID,
                DbRoomCreate {
                    id: Some(SERVER_ROOM_ID),
                    ty: RoomType::Server,
                    welcome_channel_id: None,
                },
            )
            .await?;
    }

    match &args.command {
        cli::Command::Serve {} => serve(state).await?,
        cli::Command::Check {} => check(state).await?,
        cli::Command::GcMedia {} => gc_media(state).await?,
        cli::Command::GcMessages {} => gc_messages(state).await?,
        cli::Command::GcSession {} => gc_sessions(state).await?,
        cli::Command::GcAll {} => gc_all(state).await?,
        cli::Command::Register { user_id, reason } => {
            data.user_set_registered(*user_id, Some(Time::now_utc()), None)
                .await?;
            data.room_member_put(SERVER_ROOM_ID, *user_id, None, RoomMemberPut::default())
                .await?;
            state
                .audit_log_append(AuditLogEntry {
                    id: AuditLogEntryId::new(),
                    room_id: SERVER_ROOM_ID,
                    user_id: SERVER_USER_ID,
                    session_id: None,
                    reason: reason.to_owned(),
                    ty: AuditLogEntryType::UserRegistered { user_id: *user_id },
                })
                .await?;
            // TODO: invalidate cache
            // right now i'd need to restart backend or it would think the user is still a guest
            info!("registered!");
        }
        cli::Command::MakeAdmin { user_id } => {
            data.room_member_put(
                SERVER_ROOM_ID,
                *user_id,
                None,
                types::RoomMemberPut::default(),
            )
            .await?;
            let roles = data
                .role_list(
                    SERVER_ROOM_ID,
                    PaginationQuery {
                        from: None,
                        to: None,
                        dir: Some(types::PaginationDirection::F),
                        limit: Some(2),
                    },
                )
                .await?;
            data.role_member_put(*user_id, roles.items[1].id).await?;
        }
    }

    Ok(())
}

async fn serve_transport(transport: ListenTransport, router: axum::Router) -> Result<()> {
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

/// start the main server
async fn serve(state: Arc<ServerState>) -> Result<()> {
    info!("Starting server");

    let (router, mut api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", routes::routes())
        .with_state(state.clone())
        .split_for_parts();
    NestedTags.modify(&mut api);
    BadgeModifier.modify(&mut api);
    let router = router
        .route("/api/docs.json", get(|| async { Json(api) }))
        .route(
            "/api/docs",
            get(|| async { Html(include_str!("scalar.html")) }),
        )
        .route("/", get(|| async { "it works!" }))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 16))
        .layer(cors())
        .layer(SetSensitiveHeadersLayer::new([header::AUTHORIZATION]))
        .layer(TraceLayer::new_for_http())
        .layer(CatchPanicLayer::new())
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            "x-trace-id",
        )));

    let mut set = JoinSet::new();

    for config in &state.config.listen {
        if config.components.contains(&ListenComponent::Api) {
            let router = router.clone();
            let transport = config.transport.clone();
            info!("api listening on {}", transport);
            set.spawn(async move { serve_transport(transport, router).await });
        }
    }

    if set.is_empty() {
        error!("no components enabled for any listeners");
        return Err(Error::BadStatic("no components enabled for any listeners"));
    }

    while let Some(res) = set.join_next().await {
        res.unwrap()?;
    }

    Ok(())
}

/// check config
async fn check(_state: Arc<ServerState>) -> Result<()> {
    info!("done checking");
    Ok(())
}

async fn gc_media(state: Arc<ServerState>) -> Result<()> {
    info!("starting media garbage collection");

    info!("finding items...");
    let result = sqlx::query_file!("sql/gc_media.sql")
        .execute(&state.pool)
        .await?;
    info!("found {} items to delete", result.rows_affected());

    loop {
        let rows = sqlx::query!("select id from media where deleted_at is not null limit 50")
            .fetch_all(&state.pool)
            .await?;
        let mut tx = state.pool.begin().await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let items = state
                .blobs
                .list_with(&format!("media/{}/", row.id))
                .recursive(true)
                .await?;
            for item in items {
                if item.metadata().is_file() {
                    state.blobs.delete(item.path()).await?;
                }
            }
            sqlx::query!("delete from media where id = $1", row.id)
                .execute(&mut *tx)
                .await?;
            info!("delete {}", row.id);
        }
        tx.commit().await?;
    }

    Ok(())
}

async fn gc_messages(state: Arc<ServerState>) -> Result<()> {
    info!("starting message garbage collection job");

    let result = sqlx::raw_sql(include_str!("../sql/purge_messages.sql"))
        .execute(&state.pool)
        .await?;
    info!("done; {} rows affected", result.rows_affected());

    Ok(())
}

async fn gc_sessions(state: Arc<ServerState>) -> Result<()> {
    info!("starting session garbage collection job");

    let result = sqlx::raw_sql(include_str!("../sql/purge_sessions.sql"))
        .execute(&state.pool)
        .await?;
    info!("done; {} rows affected", result.rows_affected());

    Ok(())
}

async fn gc_all(state: Arc<ServerState>) -> Result<()> {
    info!("garbage collecting everything");
    gc_media(state.clone()).await?;
    gc_messages(state.clone()).await?;
    gc_sessions(state.clone()).await?;
    Ok(())
}
