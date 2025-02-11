use std::{cmp::Ordering, sync::Arc};

use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use futures_util::StreamExt;
use tokio::io::{AsyncSeekExt, AsyncWriteExt, BufWriter};
use tracing::{debug, info, trace};
use types::UserId;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    error::{Error, Result},
    types::{Media, MediaCreate, MediaCreated, MediaId, MediaUpload},
    ServerState,
};

use super::util::Auth;

const MAX_SIZE: u64 = 1024 * 1024 * 16;

/// Media create
///
/// Create a new url to upload media to. Use the media upload endpoint for actually uploading media. Media not referenced/used in other api calls will be removed after a period of time.
#[utoipa::path(
    post,
    path = "/media",
    tags = ["media"],
    responses(
        (status = StatusCode::CREATED, description = "Create media success", body = MediaCreated)
    )
)]
async fn media_create(
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(r): Json<MediaCreate>,
) -> Result<impl IntoResponse> {
    if r.size > MAX_SIZE {
        return Err(Error::TooBig);
    }

    use async_tempfile::TempFile;
    let media_id = MediaId(uuid::Uuid::now_v7());
    let temp_file = TempFile::new().await.expect("failed to create temp file!");
    let temp_writer = BufWriter::new(temp_file.open_rw().await?);
    trace!("create temp_file {:?}", temp_file.file_path());
    let upload_url = Some(
        s.config()
            .base_url
            .join(&format!("/api/v1/media/{}", media_id))?,
    );
    s.uploads.insert(
        media_id,
        MediaUpload {
            create: r.clone(),
            user_id,
            temp_file,
            temp_writer,
        },
    );
    let res = MediaCreated {
        media_id,
        upload_url,
    };
    let mut res_headers = HeaderMap::new();
    res_headers.insert("upload-length", r.size.into());
    res_headers.insert("upload-offset", 0.into());
    Ok((StatusCode::CREATED, res_headers, Json(res)))
}

/// Media upload
#[utoipa::path(
    patch,
    path = "/media/{media_id}",
    tags = ["media"],
    params(("media_id", description = "Media id")),
    request_body = Vec<u8>,
    responses(
        (status = NO_CONTENT, description = "Upload part success"),
        (status = OK, body = Media, description = "Upload done"),
    )
)]
async fn media_upload(
    Path(media_id): Path<MediaId>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    headers: HeaderMap,
    body: Body,
) -> Result<impl IntoResponse> {
    let mut up = s.uploads.get_mut(&media_id).ok_or(Error::NotFound)?;
    if up.user_id != user_id {
        return Err(Error::NotFound);
    }
    debug!(
        "continue upload for {}, file {:?}",
        media_id,
        up.temp_file.file_path()
    );
    let stat = up.temp_file.metadata().await?;
    let current_size = stat.len();
    let current_off: u64 = headers
        .get("upload-offset")
        .ok_or(Error::BadHeader)?
        .to_str()?
        .parse()?;
    let part_length: u64 = headers
        .get("content-length")
        .ok_or(Error::BadHeader)?
        .to_str()?
        .parse()?;
    if current_size != current_off {
        return Err(Error::CantOverwrite);
    }
    if current_size + part_length > up.create.size {
        return Err(Error::TooBig);
    }
    up.temp_file
        .seek(std::io::SeekFrom::Start(current_off))
        .await?;
    let mut stream = body.into_data_stream();
    let mut end_size = current_off;
    while let Some(Ok(chunk)) = stream.next().await {
        up.temp_writer.write_all(&chunk).await?;
        end_size += chunk.len() as u64
    }
    info!("finished stream upload end_size={}", end_size);

    match end_size.cmp(&up.create.size) {
        Ordering::Greater => {
            let p = up.temp_file.file_path().to_owned();
            s.uploads.remove(&media_id);
            tokio::fs::remove_file(p).await?;
            Err(Error::TooBig)
        }
        Ordering::Equal => {
            trace!("flush media");
            up.temp_writer.flush().await?;
            trace!("flushed media");
            drop(up);
            trace!("dropped upload");
            let (_, up) = s
                .uploads
                .remove(&media_id)
                .expect("it was there a few milliseconds ago");
            trace!("processing upload");
            let mut media = process_upload(up, media_id, user_id, s.clone()).await?;
            debug!("finished processing media");
            media.url = s.presign(&media.url).await?;
            let mut headers = HeaderMap::new();
            headers.insert("upload-offset", end_size.into());
            headers.insert("upload-length", media.size.into());
            Ok((StatusCode::OK, headers, Json(Some(media))))
        }
        Ordering::Less => {
            let mut headers = HeaderMap::new();
            headers.insert("upload-offset", end_size.into());
            headers.insert("upload-length", up.create.size.into());
            Ok((StatusCode::NO_CONTENT, headers, Json(None)))
        }
    }
}

/// Media get
// TODO: restrict media visibility? or make it always public?
#[utoipa::path(
    get,
    path = "/media/{media_id}",
    tags = ["media"],
    params(("media_id", description = "Media id")),
    responses(
        (status = OK, body = Media, description = "Success"),
    )
)]
async fn media_get(
    Path((media_id,)): Path<(MediaId,)>,
    Auth(_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let mut media = s.data().media_select(media_id).await?;
    media.url = s.presign(&media.url).await?;
    Ok(Json(media))
}

/// Media check
///
/// Get headers useful for resuming an upload
#[utoipa::path(
    head,
    path = "/media/{media_id}",
    tags = ["media"],
    params(("media_id", description = "Media id")),
    responses(
        (status = NO_CONTENT, description = "no content", headers(("upload-length" = u64), ("upload-offset" = u64))),
    )
)]
async fn media_check(
    Path(media_id): Path<MediaId>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    if let Some(up) = s.uploads.get_mut(&media_id) {
        if up.user_id == user_id {
            let mut headers = HeaderMap::new();
            headers.insert("upload-offset", up.temp_file.metadata().await?.len().into());
            headers.insert("upload-length", up.create.size.into());
            return Ok((StatusCode::NO_CONTENT, headers));
        }
    }
    let media = s.data().media_select(media_id).await?;
    let mut headers = HeaderMap::new();
    headers.insert("upload-offset", media.size.into());
    headers.insert("upload-length", media.size.into());
    Ok((StatusCode::NO_CONTENT, headers))
}

/// Media delete
///
/// Delete unlinked media. If its linked to a message, delete that message instead.
#[utoipa::path(
    delete,
    path = "/media/{media_id}",
    tags = ["media"],
    params(("media_id", description = "Media id")),
    responses(
        (status = NO_CONTENT, description = "no content"),
        (status = CONFLICT, description = "media is linked to another resource (ie. a message)"),
    )
)]
async fn media_delete(
    Path(media_id): Path<MediaId>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    if let Some(up) = s.uploads.get_mut(&media_id) {
        if up.user_id == user_id {
            s.uploads.remove(&media_id);
        }
        Ok(StatusCode::NO_CONTENT)
    } else {
        let links = s.data().media_link_select(media_id).await?;
        if links.is_empty() {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Ok(StatusCode::CONFLICT)
        }
    }
}

async fn process_upload(
    up: MediaUpload,
    media_id: MediaId,
    user_id: UserId,
    s: Arc<ServerState>,
) -> Result<Media> {
    let p = up.temp_file.file_path().to_owned();
    let url = format!("media/{media_id}");
    let services = s.services();
    let (meta, mime) = services.media.get_metadata_and_mime(&p).await?;
    debug!("finish upload for {}, mime {}", media_id, mime);
    let upload_s3 = async {
        // TODO: stream upload
        let bytes = tokio::fs::read(&p).await?;
        s.blobs()
            .write_with(&url, bytes)
            .cache_control("public, max-age=604800, immutable, stale-while-revalidate=86400")
            // FIXME: sometimes this fails with "failed to parse header"
            // .content_type(&mime)
            .await?;
        Result::Ok(())
    };
    upload_s3.await?;
    info!("uploaded {} bytes to s3", up.create.size);
    let media = s
        .data()
        .media_insert(
            user_id,
            Media {
                alt: up.create.alt.clone(),
                id: media_id,
                filename: up.create.filename.clone(),
                url,
                source_url: None,
                thumbnail_url: None,
                mime,
                size: up.create.size,
                height: meta.and_then(|m| m.height),
                width: meta.and_then(|m| m.width),
                duration: meta.and_then(|m| m.duration),
            },
        )
        .await?;
    Ok(media)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(media_create))
        .routes(routes!(media_upload))
        .routes(routes!(media_get))
        .routes(routes!(media_check))
        .routes(routes!(media_delete))
}
