use std::{fmt, fs, path::Path};

use anyhow::{Context, Result};
use common::v2::types::{
    MediaId,
    media::{Media, MediaCreate, MediaCreateSource, MediaDoneParams},
};
use serde::Serialize;

use crate::auth;
use crate::config::LyConfig;

/// result of `ly media upload` and `ly media get`
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct MediaResult(pub Media);

impl fmt::Display for MediaResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let media = &self.0;
        write!(
            f,
            "{} ({:?}): {} ({} bytes)",
            media.id, media.status, media.filename, media.size
        )
    }
}

/// upload a file
pub async fn upload(
    config: &mut LyConfig,
    login: Option<&str>,
    path: &Path,
    alt: Option<String>,
    strip_exif: bool,
) -> Result<MediaResult> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read media file {}", path.display()))?;
    let filename = path
        .file_name()
        .with_context(|| format!("{} has no file name", path.display()))?
        .to_string_lossy()
        .into_owned();
    let size = bytes.len() as u64;

    let http = auth::client(config, login).await?;
    let created = http
        .media_create(&MediaCreate {
            strip_exif,
            alt,
            source: MediaCreateSource::Upload {
                filename,
                size: Some(size),
            },
        })
        .await?;

    http.media_upload(&created, bytes).await?;
    let media = http
        .media_done(
            created.media_id,
            &MediaDoneParams {
                process_async: false,
            },
        )
        .await?
        .context("media is still processing, try getting it again later")?;

    Ok(MediaResult(media))
}

/// get media metadata
pub async fn get(
    config: &mut LyConfig,
    login: Option<&str>,
    media_id: MediaId,
) -> Result<MediaResult> {
    let http = auth::client(config, login).await?;
    let media = http.media_get(media_id).await?;

    Ok(MediaResult(media))
}
