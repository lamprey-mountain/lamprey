use std::{path::Path, process::Stdio};

use tokio::process::Command;
use tracing::error;

use crate::{error::Error, Result};

async fn run_ffmpeg_with_timeout(cmd: &mut Command) -> Result<std::process::Output> {
    tokio::time::timeout(std::time::Duration::from_secs(10), cmd.output())
        .await
        .map_err(|_| Error::Ffmpeg)?
        .map_err(Into::into)
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
    .stderr(Stdio::inherit());

    let out = run_ffmpeg_with_timeout(&mut cmd).await?;
    // HACK: currently, this ffmpeg command technically works but always gives error output
    // if cmd.status.success() {
    if !out.stdout.is_empty() {
        Ok(out.stdout)
    } else {
        error!(
            stderr = String::from_utf8_lossy(&out.stderr).to_string(),
            stdout = String::from_utf8_lossy(&out.stdout).to_string(),
            "extract attachment failed",
        );
        Err(Error::Ffmpeg)
    }
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
        .stderr(Stdio::inherit());

    let out = run_ffmpeg_with_timeout(&mut cmd).await?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        error!(
            stderr = String::from_utf8_lossy(&out.stderr).to_string(),
            stdout = String::from_utf8_lossy(&out.stdout).to_string(),
            "extract stream failed",
        );
        Err(Error::Ffmpeg)
    }
}

pub async fn generate_thumb(path: &Path) -> Result<Vec<u8>> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-i"])
        .arg(path)
        .args(["-vf", "thumbnail", "-frames:v", "1", "-f", "webp", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    let out = run_ffmpeg_with_timeout(&mut cmd).await?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        error!(
            stderr = String::from_utf8_lossy(&out.stderr).to_string(),
            stdout = String::from_utf8_lossy(&out.stdout).to_string(),
            "generate thumb failed",
        );
        Err(Error::Ffmpeg)
    }
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
        .stderr(Stdio::inherit());

    let out = run_ffmpeg_with_timeout(&mut cmd).await?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        error!(
            stderr = String::from_utf8_lossy(&out.stderr).to_string(),
            stdout = String::from_utf8_lossy(&out.stdout).to_string(),
            "strip metadata failed",
        );
        Err(Error::Ffmpeg)
    }
}
