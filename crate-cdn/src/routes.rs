use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use common::v1::types::{EmojiId, MediaId};
use serde::Deserialize;
use std::io::Cursor;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{data::lookup_emoji, error::Error, AppState};

#[utoipa::path(get, path = "/media/{media_id}")]
/// Fetch media
///
/// download a piece of media
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
    /// if None, fetch the original thumbnail (eg. a video may have an embedded thumbnail)
    size: Option<u32>,
}

#[utoipa::path(get, path = "/thumb/{media_id}")]
/// Fetch thumbnail
///
/// get a thumbnail for a piece of media
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

#[utoipa::path(get, path = "/emoji/{emoji_id}")]
/// Fetch emoji
///
/// directly get an emoji's thumbnail
async fn get_emoji(
    State(state): State<AppState>,
    Path(emoji_id): Path<EmojiId>,
    Query(query): Query<ThumbQuery>,
) -> Result<impl IntoResponse, Error> {
    let media_id = lookup_emoji(&state.db, emoji_id).await?;
    get_thumb(State(state), Path(media_id), Query(query)).await
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_media))
        .routes(routes!(get_thumb))
        .routes(routes!(get_emoji))
}
