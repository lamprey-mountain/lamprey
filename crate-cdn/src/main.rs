use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use common::v1::types::{EmojiId, MediaId};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use opendal::Operator;
use serde::Deserialize;
use sqlx::query_scalar;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{io::Cursor, net::SocketAddr, str::FromStr, sync::Arc};

use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub rust_log: String,
    pub database_url: String,
    pub s3: ConfigS3,
    pub thumb_sizes: Vec<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigS3 {
    pub bucket: String,
    pub endpoint: Url,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Clone)]
struct AppState {
    db: PgPool,
    s3: Operator,
    config: Arc<Config>,
}

#[derive(Debug)]
#[allow(dead_code)] // TEMP
enum Error {
    NotFound,
    BadRequest,
    Database(sqlx::Error),
    ImageError(image::ImageError),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::NotFound => StatusCode::NOT_FOUND.into_response(),
            Error::BadRequest => StatusCode::BAD_REQUEST.into_response(),
            Error::Database(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            Error::ImageError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

impl From<opendal::Error> for Error {
    fn from(_value: opendal::Error) -> Self {
        Error::NotFound
    }
}

impl From<image::ImageError> for Error {
    fn from(value: image::ImageError) -> Self {
        Error::ImageError(value)
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Error::NotFound,
            _ => Error::Database(err),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config: Config = Figment::new()
        .merge(Toml::file("cdn.toml"))
        .merge(Env::raw())
        .extract()?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_str(&config.rust_log)?)
        .init();

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    let builder = opendal::services::S3::default()
        .bucket(&config.s3.bucket)
        .endpoint(config.s3.endpoint.as_str())
        .region(&config.s3.region)
        .access_key_id(&config.s3.access_key_id)
        .secret_access_key(&config.s3.secret_access_key);

    let s3 = Operator::new(builder)?.finish();

    let state = AppState {
        db,
        s3,
        config: Arc::new(config),
    };

    let app = Router::new()
        .route("/media/{media_id}", get(get_media))
        .route("/thumb/{media_id}", get(get_thumb))
        .route("/emoji/{emoji_id}", get(get_emoji))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 4001));
    info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_media(
    State(state): State<AppState>,
    Path(media_id): Path<MediaId>,
) -> Result<impl IntoResponse, Error> {
    let path = format!("/media/{}", media_id);
    let data = state.s3.read(&path).await?;
    Ok(data.to_bytes())
}

#[derive(Deserialize)]
struct ThumbQuery {
    size: Option<u32>,
}

async fn get_thumb(
    State(state): State<AppState>,
    Path(media_id): Path<MediaId>,
    Query(query): Query<ThumbQuery>,
) -> Result<impl IntoResponse, Error> {
    let size = query.size.unwrap_or(64);
    if !state.config.thumb_sizes.contains(&size) {
        return Err(Error::BadRequest);
    }

    let thumb_path = format!("/thumb/{}/{}x{}.webp", media_id, size, size);

    if state.s3.exists(&thumb_path).await.unwrap_or(false) {
        let data = state.s3.read(&thumb_path).await?;
        return Ok(data.to_bytes());
    }

    let media_path = format!("/media/{}", media_id);
    let media_data = state.s3.read(&media_path).await?.to_bytes();

    let image = image::load_from_memory(&media_data)?;
    let thumbnail = image.thumbnail(size, size);

    let mut buf = Cursor::new(Vec::new());
    thumbnail.write_to(&mut buf, image::ImageFormat::WebP)?;
    state
        .s3
        .write(&thumb_path, buf.clone().into_inner())
        .await?;

    Ok(axum::body::Bytes::from(buf.into_inner()))
}

async fn get_emoji(
    State(state): State<AppState>,
    Path(emoji_id): Path<EmojiId>,
    Query(query): Query<ThumbQuery>,
) -> Result<impl IntoResponse, Error> {
    let media_id: MediaId =
        query_scalar!("SELECT media_id FROM custom_emoji WHERE id = $1", *emoji_id)
            .fetch_one(&state.db)
            .await?
            .into();

    get_thumb(State(state), Path(media_id), Query(query)).await
}
