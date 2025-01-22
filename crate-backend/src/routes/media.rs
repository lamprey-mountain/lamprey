use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use tokio::{
    io::{AsyncSeekExt, AsyncWriteExt},
    process::Command,
};
use tracing::debug;
use url::Url;
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
        (status = StatusCode::CREATED, description = "Get room success", body = MediaCreated)
    )
)]
async fn media_create(
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
    Json(r): Json<MediaCreate>,
) -> Result<(StatusCode, HeaderMap, Json<MediaCreated>)> {
    if r.size > MAX_SIZE {
        return Err(Error::TooBig);
    }

    use async_tempfile::TempFile;
    let user_id = session.user_id;
    let media_id = MediaId(uuid::Uuid::now_v7());
    let temp_file = TempFile::new().await.expect("failed to create temp file!");
    let upload_url = Some(
        Url::parse(&format!(
            "https://chat.celery.eu.org/api/v1/media/{media_id}"
        ))
        .expect("somehow constructed invalid url"),
    );
    s.uploads.insert(
        media_id,
        MediaUpload {
            create: r.clone(),
            user_id,
            temp_file,
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
// TODO: stream
#[utoipa::path(
    patch,
    path = "/media/{media_id}",
    tags = ["media"],
    params(("media_id", description = "Media id")),
    request_body = Vec<u8>,
    responses(
        (status = NO_CONTENT, description = "Upload success" ),
        (status = OK, description = "Upload done" ),
    )
)]
async fn media_upload(
    Path((media_id,)): Path<(MediaId,)>,
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, HeaderMap, Json<Option<Media>>)> {
    let mut up = s.uploads.get_mut(&media_id).ok_or(Error::NotFound)?;
    if up.user_id != session.user_id {
        return Err(Error::NotFound);
    }
    debug!("continue upload for {}, file {:?}", media_id, up.temp_file.file_path());
    let stat = up.temp_file.metadata().await?;
    let current_size = stat.len();
    let current_off: u64 = headers
        .get("upload-offset")
        .ok_or(Error::BadHeader)?
        .to_str()?
        .parse()?;
    if current_size != current_off {
        return Err(Error::CantOverwrite);
    }
    if current_size + current_off > up.create.size {
        return Err(Error::TooBig);
    }
    up.temp_file.seek(std::io::SeekFrom::End(0)).await?;
    let end_size = current_off + up.temp_file.write(&body).await? as u64;
    if end_size > up.create.size {
        let p = up.temp_file.file_path().to_owned();
        s.uploads.remove(&media_id);
        tokio::fs::remove_file(p).await?;
        Err(Error::TooBig)
    } else if end_size == up.create.size {
        let p = up.temp_file.file_path().to_owned();
        let url = format!("media/{media_id}");
        let (meta, mime) = tokio::try_join!(get_metadata(&p), get_mime_type(&p))?;
        debug!("finish upload for {}, mime {}", media_id, mime);
        let upload_s3 = async {
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
        let user_id = session.user_id;
        let mut media = s
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
                    height: meta.height,
                    width: meta.width,
                    duration: meta.duration,
                },
            )
            .await?;
        let size = up.create.size;
        drop(up);
        s
            .uploads
            .remove(&media_id)
            .expect("it was there a few milliseconds ago");
        media.url = s.presign(&media.url).await?;
        let mut headers = HeaderMap::new();
        headers.insert("upload-offset", end_size.into());
        headers.insert("upload-length", size.into());
        Ok((StatusCode::OK, headers, Json(Some(media))))
    } else {
        let mut headers = HeaderMap::new();
        headers.insert("upload-offset", end_size.into());
        headers.insert("upload-length", up.create.size.into());
        Ok((StatusCode::NO_CONTENT, headers, Json(None)))
    }
}

/// media get
// todo: restrict media visibility? or make it always public?
#[utoipa::path(
    get,
    path = "/media/{media_id}",
    tags = ["media"],
    params(("media_id", description = "Media id")),
    responses(
        (status = OK, description = "Success" ),
    )
)]
async fn media_get(
    Path((media_id,)): Path<(MediaId,)>,
    Auth(_session): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<Json<Media>> {
    let mut media = s.data().media_select(media_id).await?;
    media.url = s.presign(&media.url).await?;
    Ok(Json(media))
}

/// media head
#[utoipa::path(
    head,
    path = "/media/{media_id}",
    tags = ["media"],
    params(("media_id", description = "Media id")),
    responses(
        (status = NO_CONTENT, description = "no content"),
    )
)]
async fn media_check(
    Path((media_id,)): Path<(MediaId,)>,
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<(StatusCode, HeaderMap)> {
    if let Some(up) = s.uploads.get_mut(&media_id) {
        if up.user_id == session.user_id {
            let mut headers = HeaderMap::new();
            headers.insert("upload-offset", up.temp_file.metadata().await?.len().into());
            headers.insert("upload-length", up.create.size.into());
            return Ok((StatusCode::NO_CONTENT, headers))
        }
    }
    let media = s.data().media_select(media_id).await?;
    let mut headers = HeaderMap::new();
    headers.insert("upload-offset", media.size.into());
    headers.insert("upload-length", media.size.into());
    Ok((StatusCode::NO_CONTENT, headers))
}

// 	app.openAPIRegistry.registerPath(MediaCheck);

// 	app.openapi(withAuth(MediaGet), async (c) => {
// 		const user_id = c.get("user_id");
// 		const media_id = c.req.param("media_id");
// 		// extremely dubious
// 		if (c.req.method === "HEAD") {
// 			const up = uploads.get(media_id);
// 			console.log({ uploads })
// 			if (!up) return c.json({ error: "not found" }, 404);
// 			if (up.user_id !== user_id) return c.json({ error: "not found" }, 404);
// 			const stat = await Deno.stat(up.temp_file);
// 			return new Response(null, {
// 				status: 204,
// 				headers: {
// 					"Upload-Offset": stat.size.toString(),
// 					"Upload-Length": up.size.toString(),
// 				},
// 			}) as any;
// 		} else {
// 			const media = await data.mediaSelect(media_id);
// 			if (!media) return c.json({ error: "not found" }, 404);
// 			media.url = await blobs.presignedGetUrl(media.url);
// 			return c.json(media, 200);
// 		}
// 	});

struct Metadata {
    height: Option<u64>,
    width: Option<u64>,
    duration: Option<u64>,
}

mod ffprobe {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Metadata {
        pub streams: Vec<Stream>,
        pub format: Format,
    }

    #[derive(Debug, Deserialize)]
    pub struct Format {
        pub duration: Option<String>,
        // #[serde(default)]
        // pub tags: HashMap<String, String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Stream {
        // pub codec_name: String,
        // pub codec_type: String,
        pub width: Option<u64>,
        pub height: Option<u64>,
        // #[serde(default)]
        // pub tags: HashMap<String, String>,
        pub disposition: Disposition,
    }

    #[derive(Debug, Deserialize)]
    pub struct Disposition {
        pub default: u8,
    }
}

async fn get_metadata(file: &std::path::Path) -> Result<Metadata> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-of",
            "json",
            "-show_format",
            "-show_streams",
            "-i",
        ])
        .arg(file)
        .output()
        .await?;
    let json: ffprobe::Metadata = serde_json::from_slice(&out.stdout)?;
    let duration: Option<f64> = match json.format.duration {
        Some(s) => Some(s.parse::<f64>()? * 1000.),
        None => None,
    };
    let dims = json
        .streams
        .iter()
        .find(|i| i.disposition.default == 1 && i.width.is_some())
        .or_else(|| json.streams.iter().find(|i| i.width.is_some()));
    Ok(Metadata {
        height: dims.and_then(|i| i.height),
        width: dims.and_then(|i| i.width),
        duration: duration.map(|i| i as u64),
    })
}

async fn get_mime_type(file: &std::path::Path) -> Result<String> {
    let out = Command::new("file").arg("-ib").arg(file).output().await?;
    let mime = String::from_utf8(out.stdout).expect("file has failed me");
    Ok(mime)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(media_create))
        .routes(routes!(media_upload))
        .routes(routes!(media_get))
        .routes(routes!(media_check))
}
