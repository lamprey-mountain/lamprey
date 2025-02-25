use std::{
    collections::HashMap,
    ops::Deref,
    str::FromStr,
    sync::{Arc, Weak},
    time::Duration,
};

use ::types::{Media, Message, RoomId, ThreadId, UrlEmbed, UserId};
use axum::{extract::DefaultBodyLimit, response::Html, routing::get, Json};
use dashmap::DashMap;
use data::{postgres::Postgres, Data};
use figment::providers::{Env, Format, Toml};
use http::header;
use ipnet::IpNet;
use moka::future::Cache;
use opendal::layers::LoggingLayer;
use serde::Deserialize;
use services::Services;
use sqlx::{postgres::PgPoolOptions, PgPool};
use sync::Connection;
use tokio::sync::broadcast::Sender;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::debug;
use tracing_subscriber::EnvFilter;
use types::MessageSync;
use url::Url;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

pub mod data;
pub mod error;
mod routes;
pub mod services;
mod sync;
pub mod types;

use error::Result;

#[derive(OpenApi)]
#[openapi(components(schemas(
    types::Room,
    types::RoomPatch,
    types::User,
    types::Thread,
    types::ThreadPatch,
    types::Message,
    types::RoomMember,
    types::Role,
)))]
struct ApiDoc;

pub struct ServerStateInner {
    pub config: Config,
    pub pool: PgPool,
    pub services: Weak<Services>,

    // this is fine probably
    pub sushi: Sender<MessageSync>,
    // channel_user: Arc<DashMap<UserId, (Sender<MessageServer>, Receiver<MessageServer>)>>,

    // TODO: write a wrapper around this
    pub blobs: opendal::Operator,

    cache_presigned: Cache<Url, Url>,
}

// newly signed urls last for 24 hours = 1 day
const PRESIGNED_URL_LIFETIME: Duration = Duration::from_secs(60 * 60 * 24);

// the server will only return urls that are valid for at least 8 hours
const PRESIGNED_MIN_LIFETIME: Duration = Duration::from_secs(60 * 60 * 8);

pub struct ServerState {
    pub inner: Arc<ServerStateInner>,
    pub services: Arc<Services>,

    // TODO: limit number of connections per user, clean up old/unused entries
    pub syncers: Arc<DashMap<String, Connection>>,
}

impl ServerStateInner {
    fn data(&self) -> Box<dyn Data> {
        Box::new(Postgres {
            pool: self.pool.clone(),
        })
    }

    fn services(&self) -> Arc<Services> {
        self.services
            .upgrade()
            .expect("services should always exist while serverstateinner is alive")
    }

    // fn acquire_data(&self) -> Box<dyn Data> {
    //     Box::new(Postgres {
    //         pool: self.pool.clone(),
    //     })
    // }

    async fn broadcast_room(
        &self,
        room_id: RoomId,
        user_id: UserId,
        reason: Option<String>,
        msg: MessageSync,
    ) -> Result<()> {
        if msg.is_room_audit_loggable() {
            self.data()
                .audit_logs_room_append(room_id, user_id, reason, msg.clone())
                .await?;
        }
        let _ = self.sushi.send(msg);
        Ok(())
    }

    async fn broadcast_thread(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        reason: Option<String>,
        msg: MessageSync,
    ) -> Result<()> {
        if msg.is_room_audit_loggable() {
            let thread = self
                .services()
                .threads
                .get(thread_id, Some(user_id))
                .await?;
            self.broadcast_room(thread.room_id, user_id, reason, msg)
                .await?;
        } else {
            let _ = self.sushi.send(msg);
        }
        Ok(())
    }

    fn broadcast(&self, msg: MessageSync) -> Result<()> {
        let _ = self.sushi.send(msg);
        Ok(())
    }

    fn get_s3_url(&self, path: &str) -> Result<Url> {
        let mut u = Url::parse("s3://")?;
        u.set_host(Some(&self.config.s3.bucket))?;
        u.set_path(path);
        Ok(u)
    }

    /// presigns every relevant url in a piece of media
    async fn presign(&self, media: &mut Media) -> Result<()> {
        for t in media.all_tracks_mut() {
            t.url = self
                .cache_presigned
                .try_get_with(t.url.to_owned(), async {
                    let signed: Url = self
                        .blobs
                        .presign_read(t.url.path(), PRESIGNED_URL_LIFETIME)
                        .await?
                        .uri()
                        .to_string()
                        .parse()?;
                    crate::Result::Ok(signed)
                })
                .await
                .map_err(|err| err.fake_clone())?;
        }
        Ok(())
    }

    /// presigns every relevant url in a UrlEmbed
    async fn presign_url_embed(&self, embed: &mut UrlEmbed) -> Result<()> {
        if let Some(m) = &mut embed.media {
            self.presign(m).await?;
        }
        if let Some(m) = &mut embed.author_avatar {
            self.presign(m).await?;
        }
        if let Some(m) = &mut embed.site_avatar {
            self.presign(m).await?;
        }
        Ok(())
    }

    /// presigns every relevant url in a Message
    async fn presign_message(&self, message: &mut Message) -> Result<()> {
        for media in &mut message.attachments {
            self.presign(media).await?;
        }
        for emb in &mut message.embeds {
            self.presign_url_embed(emb).await?;
        }
        Ok(())
    }
}

impl ServerState {
    fn new(config: Config, pool: PgPool, blobs: opendal::Operator) -> Self {
        // a bit hacky for now since i need to work around the existing ServerState
        // though i probably need some way to access global state/services from within them anyways
        let services = Arc::new_cyclic(|weak| {
            let inner = Arc::new(ServerStateInner {
                config,
                pool,
                services: weak.to_owned(),
                blobs,

                // maybe i should increase the limit at some point? or make it unlimited?
                sushi: tokio::sync::broadcast::channel(100).0,
                cache_presigned: Cache::builder()
                    .max_capacity(1_000_000)
                    .time_to_live(PRESIGNED_URL_LIFETIME - PRESIGNED_MIN_LIFETIME)
                    .build(),
            });
            Services::new(inner.clone())
        });
        Self {
            inner: services.state.clone(),
            syncers: Arc::new(DashMap::new()),
            // channel_user: Arc::new(DashMap::new()),
            services,
        }
    }

    fn config(&self) -> &Config {
        &self.inner.config
    }

    fn data(&self) -> Box<dyn Data> {
        self.inner.data()
    }

    fn services(self: &Arc<Self>) -> Arc<Services> {
        self.services.clone()
    }
}

impl Deref for ServerState {
    type Target = ServerStateInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    rust_log: String,
    database_url: String,
    base_url: Url,
    s3: ConfigS3,
    oauth_provider: HashMap<String, ConfigOauthProvider>,
    url_preview: ConfigUrlPreview,
}

#[derive(Debug, Deserialize)]
pub struct ConfigS3 {
    bucket: String,
    endpoint: Url,
    region: String,
    access_key_id: String,
    secret_access_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigOauthProvider {
    client_id: String,
    client_secret: String,
    authorization_url: String,
    token_url: String,
    revocation_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigUrlPreview {
    deny: Vec<IpNet>,
}

fn cors() -> CorsLayer {
    use header::{HeaderName, AUTHORIZATION, CONTENT_TYPE};
    const UPLOAD_OFFSET: HeaderName = HeaderName::from_static("upload-offset");
    const UPLOAD_LENGTH: HeaderName = HeaderName::from_static("upload-length");
    CorsLayer::very_permissive()
        .expose_headers([CONTENT_TYPE, UPLOAD_OFFSET, UPLOAD_LENGTH])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE, UPLOAD_OFFSET, UPLOAD_LENGTH])
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    let config: Config = figment::Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    debug!("Starting with config: {:#?}", config);

    let sub = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_str(&config.rust_log)?)
        .finish();
    tracing::subscriber::set_global_default(sub)?;

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

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", routes::routes())
        .with_state(state)
        .split_for_parts();
    let router = router
        .route("/api/docs.json", get(|| async { Json(api) }))
        .route(
            "/api/docs",
            get(|| async { Html(include_str!("scalar.html")) }),
        )
        .layer(cors())
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(1024 * 1024 * 16));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await?;
    axum::serve(listener, router).await?;
    Ok(())
}
