use std::sync::Arc;

use async_tempfile::TempFile;
use axum::{
    body::Body,
    extract::{Path, State},
};
use common::v1::types::MediaId;
use http::{HeaderMap, StatusCode};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
};
use tokio_util::io::ReaderStream;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    error::{Error, Result},
    ffmpeg,
    routes::util::{build_headers, ContentInfo},
    AppState,
};

async fn gifv_response(
    s: AppState,
    media_id: MediaId,
    headers: HeaderMap,
    with_body: bool,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    let media = s.lookup_media(media_id).await?;
    if media.source.mime.as_str() != "image/gif" {
        return Err(Error::BadRequest);
    }

    let pre_header_info = build_headers(
        &headers,
        &ContentInfo::Gifv {
            media: &media,
            content_length: None,
        },
    )?;

    if pre_header_info.unmodified {
        return Ok((
            StatusCode::NOT_MODIFIED,
            pre_header_info.headers,
            Body::empty(),
        ));
    }

    let gifv_path = format!("/media/{media_id}/gifv");

    if s.s3.exists(&gifv_path).await? {
        let meta = s.s3.stat(&gifv_path).await?;
        let content_length = meta.content_length();
        let final_headers = build_headers(
            &headers,
            &ContentInfo::Gifv {
                media: &media,
                content_length: Some(content_length),
            },
        )?;

        let status = if final_headers.range.is_some() {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        };

        let body = if with_body {
            let reader = s.s3.reader(&gifv_path).await?;
            if let Some(r) = final_headers.range {
                Body::from_stream(reader.into_bytes_stream(r).await?)
            } else {
                Body::from_stream(reader.into_bytes_stream(..).await?)
            }
        } else {
            Body::empty()
        };

        return Ok((status, final_headers.headers, body));
    }

    let temp_file = s
        .pending_gifv
        .try_get_with(media_id, async {
            let source_path = format!("/media/{media_id}/file");
            let temp_in = TempFile::new().await?;
            let temp_out = TempFile::new().await?;
            let reader = s.s3.reader(&source_path).await?;
            let mut writer = temp_in.open_rw().await?;
            let chunk = reader.read(..).await?;
            writer.write_all(&chunk.to_vec()).await?;

            ffmpeg::transcode_to_webm(temp_in.file_path(), temp_out.file_path()).await?;

            let s_clone = s.s3.clone();
            let out_clone = temp_out.file_path().to_owned();
            tokio::spawn(async move {
                let mut f = File::open(out_clone).await.unwrap();
                let mut writer = s_clone.writer(&gifv_path).await.unwrap();
                let mut buf = vec![0; 1024 * 64];
                loop {
                    let n = f.read(&mut buf).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    writer.write(buf[..n].to_vec()).await.unwrap();
                }
                writer.close().await.unwrap();
            });

            Ok::<_, Error>(Arc::new(temp_out))
        })
        .await
        .map_err(|e| e.as_ref().clone())?;

    let mut f = temp_file.open_ro().await?;
    let content_length = f.metadata().await?.len();

    let final_headers = build_headers(
        &headers,
        &ContentInfo::Gifv {
            media: &media,
            content_length: Some(content_length),
        },
    )?;

    let status = if final_headers.range.is_some() {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };

    let body = if with_body {
        if let Some(range) = final_headers.range {
            let start = match range.0 {
                std::ops::Bound::Included(s) => s,
                std::ops::Bound::Excluded(s) => s + 1,
                std::ops::Bound::Unbounded => 0,
            };
            let end = match range.1 {
                std::ops::Bound::Included(e) => e + 1,
                std::ops::Bound::Excluded(e) => e,
                std::ops::Bound::Unbounded => content_length,
            };
            let len = end - start;
            f.seek(std::io::SeekFrom::Start(start)).await?;
            let mut buf = vec![0; len as usize];
            f.read_exact(&mut buf).await?;
            Body::from(buf)
        } else {
            Body::from_stream(ReaderStream::new(f))
        }
    } else {
        Body::empty()
    };

    Ok((status, final_headers.headers, body))
}

/// Fetch gifv
///
/// get a gifv for a piece of media
#[utoipa::path(get, path = "/gifv/{media_id}")]
pub async fn get_gifv(
    State(s): State<AppState>,
    Path(media_id): Path<MediaId>,
    headers: HeaderMap,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    gifv_response(s, media_id, headers, true).await
}

/// Head gifv
///
/// get headers for a gifv for a piece of media
#[utoipa::path(head, path = "/gifv/{media_id}")]
pub async fn head_gifv(
    State(s): State<AppState>,
    Path(media_id): Path<MediaId>,
    headers: HeaderMap,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    gifv_response(s, media_id, headers, false).await
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(head_gifv))
        .routes(routes!(get_gifv))
}
