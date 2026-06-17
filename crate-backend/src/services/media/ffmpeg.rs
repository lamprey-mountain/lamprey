use std::{path::Path, process::Stdio};

use tokio::process::Command;
use tracing::{error, trace};

use crate::{Result, error::Error};

async fn run_ffmpeg(cmd: &mut Command, context: &str) -> Result<Vec<u8>> {
    let out = tokio::time::timeout(std::time::Duration::from_secs(10), cmd.output())
        .await
        .map_err(|_| Error::Ffmpeg)??;

    // HACK: currently, some ffmpeg commands technically work but always gives error output. check stdout instead.
    if out.status.success() || !out.stdout.is_empty() {
        if !out.status.success() {
            trace!(
                stderr = %String::from_utf8_lossy(&out.stderr),
                stdout = %String::from_utf8_lossy(&out.stdout),
                "{context} exited with non-zero status but produced output",
            );
        }
        Ok(out.stdout)
    } else {
        error!(
            stderr = %String::from_utf8_lossy(&out.stderr),
            stdout = %String::from_utf8_lossy(&out.stdout),
            "{context} failed",
        );
        Err(Error::Ffmpeg)
    }
}

pub async fn extract_attachment(path: &Path, index: u64) -> Result<Vec<u8>> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        &format!("-dump_attachment:{}", index),
        "/dev/stdout",
        "-y",
        "-i",
    ])
    .arg(path)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    run_ffmpeg(&mut cmd, "extract attachment").await
}

pub async fn extract_stream(path: &Path, index: u64) -> Result<Vec<u8>> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-i"])
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
        .stderr(Stdio::piped());

    run_ffmpeg(&mut cmd, "extract stream").await
}

pub async fn generate_thumb(path: &Path) -> Result<Vec<u8>> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-i"])
        .arg(path)
        .args(["-vf", "thumbnail", "-frames:v", "1", "-f", "webp", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    run_ffmpeg(&mut cmd, "generate thumb").await
}

pub async fn strip_metadata(path: &Path, format: &str) -> Result<Vec<u8>> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-i"])
        .arg(path)
        .args(["-map_metadata", "-1"])
        .args(["-f", format])
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    run_ffmpeg(&mut cmd, "strip metadata").await
}
