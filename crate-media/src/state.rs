use std::{sync::Arc, time::Duration};

use common::{
    v1::types::{EmojiId, MediaId, MessageSync},
    v2::types::media::{Media, MediaStatus},
};
use moka::future::Cache;
use opendal::{layers::LoggingLayer, Operator};
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::{config::Config, data, Error, Result};

#[derive(Clone)]
pub struct AppState {
    pub(crate) db: PgPool,
    pub(crate) s3: Operator,
    pub(crate) nats: Option<async_nats::Client>,
    pub(crate) config: Arc<Config>,

    // NOTE: be careful about allowing emoji/media editing! i'd need to invalidate these caches
    pub(crate) cache_emoji: Cache<EmojiId, MediaId>,
    pub(crate) cache_media: Cache<MediaId, Media>,
    pub(crate) pending_thumbnails: Cache<(MediaId, u32, u32, bool), Vec<u8>>,
    pub(crate) pending_gifv: Cache<MediaId, Arc<async_tempfile::TempFile>>,

    pub(crate) sushi_tx: tokio::sync::broadcast::Sender<MessageSync>,
}

impl AppState {
    pub async fn init_from_config(config: Config) -> Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&config.database_url)
            .await?;

        let builder = opendal::services::S3::default()
            .bucket(&config.s3.bucket)
            .endpoint(config.s3.endpoint.as_str())
            .region(&config.s3.region)
            .access_key_id(&config.s3.access_key_id)
            .secret_access_key(&config.s3.secret_access_key);
        let s3 = Operator::new(builder)?
            .layer(LoggingLayer::default())
            .finish();

        let (sushi_tx, _) = tokio::sync::broadcast::channel(100);
        let nats = if let Some(nats_config) = &config.nats {
            let mut nats_options = async_nats::ConnectOptions::new();
            if let Some(credentials_path) = &nats_config.credentials {
                nats_options = nats_options
                    .credentials_file(credentials_path)
                    .await
                    .map_err(|e| Error::Internal(format!("NATS credentials file failed: {}", e)))?;
            }
            Some(
                async_nats::connect_with_options(&nats_config.addr, nats_options)
                    .await
                    .map_err(|e| Error::Internal(format!("NATS connect failed: {}", e)))?,
            )
        } else {
            None
        };

        let cache_media = Cache::new(config.cache_media);
        let cache_emoji = Cache::new(config.cache_emoji);

        Ok(Self {
            db,
            s3,
            nats,
            config: Arc::new(config),
            cache_emoji,
            cache_media,
            pending_thumbnails: Cache::new(0),
            pending_gifv: Cache::new(100),
            sushi_tx,
        })
    }

    pub async fn lookup_emoji(&self, emoji_id: EmojiId) -> Result<MediaId> {
        if let Some(m) = self.cache_emoji.get(&emoji_id).await {
            return Ok(m);
        }
        let m = data::lookup_emoji(&self.db, emoji_id).await?;
        self.cache_emoji.insert(emoji_id, m).await;
        Ok(m)
    }

    pub async fn ensure_media_ready(&self, media_id: MediaId, wait: bool) -> Result<Media> {
        if let Some(m) = self.cache_media.get(&media_id).await {
            return Ok(m);
        }

        let mut sub = self.sushi_tx.subscribe();

        loop {
            let (media, status) = data::lookup_media_with_status(&self.db, media_id).await?;
            if matches!(
                status,
                Some(MediaStatus::Uploaded) | Some(MediaStatus::Consumed) | None
            ) {
                self.cache_media.insert(media_id, media.clone()).await;
                return Ok(media);
            }

            if !wait {
                return Err(Error::StillProcessing);
            }

            if self.nats.is_some() {
                loop {
                    match sub.recv().await {
                        Ok(MessageSync::MediaProcessed { media: m, .. }) if m.id == media_id => {
                            let media_v2: Media = data::DbMediaData::V2(m).into();
                            self.cache_media.insert(media_id, media_v2.clone()).await;
                            return Ok(media_v2);
                        }
                        Ok(_) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            break; // Re-check DB
                        }
                        Err(_) => {
                            return Err(Error::Internal(
                                "NATS subscription ended unexpectedly".to_string(),
                            ));
                        }
                    }
                }
            } else {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

// async fn main() -> anyhow::Result<()> {
//     let config: Config = Figment::new()
//         .merge(Toml::file("cdn.toml"))
//         .merge(Env::raw())
//         .extract()?;

//     info!("starting cdn with config: {:#?}", config);

//     let state = AppState {
//         db,
//         s3,
//         nats,
//         config: Arc::new(config),
//         cache_media,
//         cache_emoji,
//         sushi_tx,
//     };

//     let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
//         .merge(routes::routes())
//         .with_state(state)
//         .split_for_parts();
//     let router = router
//         .route("/api/docs.json", get(|| async { Json(api) }))
//         .route(
//             "/api/docs",
//             get(|| async { Html(include_str!("scalar.html")) }),
//         )
//         .route("/", get(|| async { "it works!" }))
//         .layer(cors())
//         .layer(TraceLayer::new_for_http())
//         .layer(CatchPanicLayer::new())
//         .layer(PropagateHeaderLayer::new(HeaderName::from_static(
//             "x-trace-id",
//         )));
//     let listener = tokio::net::TcpListener::bind("0.0.0.0:4001").await?;
//     axum::serve(listener, router).await?;

//     Ok(())
// }
