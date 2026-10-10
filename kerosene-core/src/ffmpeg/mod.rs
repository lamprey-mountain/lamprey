use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use thiserror::Error;
use tokio::process::Command;
use tracing::error;

use crate::{config::Config, ffmpeg::metadata::MediaMetadata};

pub mod metadata;

#[derive(Debug, Default)]
pub struct Ffmpeg {
    ffmpeg_path: Option<PathBuf>,
    ffprobe_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Error)]
pub enum FfmpegError {
    /// the command could not be found
    // #[error("command {0} not found")]
    #[error("command not found")]
    CommandNotFound,
    // NOTE: maybe Io(#[from] std::io::Error) would be good enough?
    /// timed out
    #[error("timed out")]
    TimedOut,

    // TODO: better errors
    /// other
    #[error("other")]
    Other,
}

impl From<std::io::Error> for FfmpegError {
    fn from(value: std::io::Error) -> Self {
        match value.kind() {
            std::io::ErrorKind::NotFound => Self::CommandNotFound,
            // TODO: support other error kinds
            // std::io::ErrorKind::PermissionDenied => todo!(),
            // std::io::ErrorKind::IsADirectory => todo!(),
            // std::io::ErrorKind::FilesystemLoop => todo!(),
            // std::io::ErrorKind::TimedOut => todo!(),
            // std::io::ErrorKind::QuotaExceeded => todo!(),
            // std::io::ErrorKind::InvalidFilename => todo!(),
            // std::io::ErrorKind::UnexpectedEof => todo!(),
            // std::io::ErrorKind::OutOfMemory => todo!(),
            // std::io::ErrorKind::Interrupted => todo!(),
            // std::io::ErrorKind::ArgumentListTooLong => todo!(),
            _ => Self::Other,
        }
    }
}

// TODO; add doc comments
impl Ffmpeg {
    pub fn from_config(_config: &Config) -> Self {
        // TODO: use config
        Self::default()
    }

    pub fn resolved_ffmpeg_path(&self) -> &Path {
        self.ffmpeg_path.as_deref().unwrap_or(&Path::new("ffmpeg"))
    }

    pub fn resolved_ffprobe_path(&self) -> &Path {
        self.ffprobe_path
            .as_deref()
            .unwrap_or(&Path::new("ffprobe"))
    }

    pub async fn transcode_to_webm(
        &self,
        in_path: &Path,
        out_path: &Path,
    ) -> Result<(), FfmpegError> {
        // TODO: make args/parameters configurable (eg. codec, crf)
        let cmd = Command::new(self.resolved_ffmpeg_path())
            .args(["-v", "quiet", "-y", "-i"])
            .arg(in_path)
            .args([
                "-c:v",
                "libvpx-vp9",
                "-crf",
                "30",
                "-b:v",
                "0",
                "-an",
                "-f",
                "webm",
            ])
            .arg(out_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .await?;

        if cmd.status.success() {
            Ok(())
        } else {
            // TODO: parse std{err,out} and return as error?
            error!(
                stderr = String::from_utf8_lossy(&cmd.stderr).to_string(),
                stdout = String::from_utf8_lossy(&cmd.stdout).to_string(),
                "transcode failed",
            );
            Err(FfmpegError::Other)
        }
    }

    pub async fn generate_thumbnail(
        &self,
        in_path: &Path,
        out_path: &Path,
        size: u32,
        animate: bool,
    ) -> Result<(), FfmpegError> {
        let mut cmd = Command::new(self.resolved_ffmpeg_path());
        cmd.args(["-v", "quiet", "-y", "-i"]).arg(in_path);

        if animate {
            // Generate animated WebP for thumbnails
            cmd.args([
                "-vf",
                &format!("scale={size}:{size}:force_original_aspect_ratio=decrease"),
                "-loop",
                "0",
                "-f",
                "webp",
            ]);
        } else {
            // Generate static WebP (first frame)
            // WebP supports transparency natively, which avoids needing to extract a separate alpha stream for AVIF.
            cmd.args([
                "-vf",
                &format!("scale={size}:{size}:force_original_aspect_ratio=decrease"),
                "-frames:v",
                "1",
                "-f",
                "webp",
            ]);
        }

        cmd.arg(out_path);

        let output = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            error!(
                stderr = String::from_utf8_lossy(&output.stderr).to_string(),
                stdout = String::from_utf8_lossy(&output.stdout).to_string(),
                "thumbnail generation failed",
            );
            Err(FfmpegError::Other)
        }
    }

    pub async fn extract_attachment(
        &self,
        path: &Path,
        index: u64,
    ) -> Result<Vec<u8>, FfmpegError> {
        let output = Command::new(self.resolved_ffmpeg_path())
            .args([
                "-v",
                "quiet",
                &format!("-dump_attachment:{}", index),
                "/dev/stdout",
                "-y",
                "-i",
            ])
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if output.status.success() || !output.stdout.is_empty() {
            Ok(output.stdout)
        } else {
            error!(
                stderr = String::from_utf8_lossy(&output.stderr).to_string(),
                stdout = String::from_utf8_lossy(&output.stdout).to_string(),
                "extract attachment failed",
            );
            Err(FfmpegError::Other)
        }
    }

    pub async fn extract_stream(&self, path: &Path, index: u64) -> Result<Vec<u8>, FfmpegError> {
        let output = Command::new(self.resolved_ffmpeg_path())
            .args(["-v", "quiet", "-i"])
            .arg(path)
            .args([
                "-map",
                &format!("0:{}", index),
                "-f",
                "image2",
                "-c:v",
                "copy",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if output.status.success() || !output.stdout.is_empty() {
            Ok(output.stdout)
        } else {
            error!(
                stderr = String::from_utf8_lossy(&output.stderr).to_string(),
                stdout = String::from_utf8_lossy(&output.stdout).to_string(),
                "extract stream failed",
            );
            Err(FfmpegError::Other)
        }
    }

    pub async fn extract_or_generate_video_thumbnail(
        &self,
        path: &Path,
    ) -> Result<Vec<u8>, FfmpegError> {
        // TODO: better thumbnail generation logic for videos
        // eg. skip solid black/white frames at start of video, get thumb x% of the way through the video
        let output = Command::new(self.resolved_ffmpeg_path())
            .args(["-v", "quiet", "-i"])
            .arg(path)
            .args(["-vf", "thumbnail", "-frames:v", "1", "-f", "webp", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if output.status.success() || !output.stdout.is_empty() {
            Ok(output.stdout)
        } else {
            error!(
                stderr = String::from_utf8_lossy(&output.stderr).to_string(),
                stdout = String::from_utf8_lossy(&output.stdout).to_string(),
                "generate thumb failed",
            );
            Err(FfmpegError::Other)
        }
    }

    pub async fn strip_metadata(&self, path: &Path, format: &str) -> Result<Vec<u8>, FfmpegError> {
        let output = Command::new(self.resolved_ffmpeg_path())
            .args(["-v", "quiet", "-i"])
            .arg(path)
            .args(["-map_metadata", "-1"])
            .args(["-f", format])
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if output.status.success() || !output.stdout.is_empty() {
            Ok(output.stdout)
        } else {
            error!(
                stderr = String::from_utf8_lossy(&output.stderr).to_string(),
                stdout = String::from_utf8_lossy(&output.stdout).to_string(),
                "strip metadata failed",
            );
            Err(FfmpegError::Other)
        }
    }

    pub async fn extract_metadata(&self, path: &Path) -> Result<MediaMetadata, FfmpegError> {
        let out = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            Command::new("ffprobe")
                .args([
                    "-v",
                    "quiet",
                    "-of",
                    "json",
                    "-show_format",
                    "-show_streams",
                    "-i",
                ])
                .arg(path)
                .output(),
        )
        .await
        .map_err(|_| FfmpegError::TimedOut)?
        .map_err(|_| FfmpegError::Other)?;
        if out.status.success() {
            // TODO: serde error variant
            let meta = serde_json::from_slice(&out.stdout).map_err(|_| FfmpegError::Other)?;
            Ok(meta)
        } else {
            Err(FfmpegError::Other)
        }
    }
}
