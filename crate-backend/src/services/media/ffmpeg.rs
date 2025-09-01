use std::{path::Path, process::Stdio};

use tokio::process::Command;
use tracing::error;

use crate::{error::Error, Result};

pub async fn extract_attachment(path: &Path, index: u64) -> Result<Vec<u8>> {
    let cmd = Command::new("ffmpeg")
        // .args(["-v", "quiet"])
        .args([
            &format!("-dump_attachment:{}", index),
            "/dev/stdout",
            "-y",
            "-i",
        ])
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .await?;
    // HACK: currently, this ffmpeg command technically works but always gives error output
    // if cmd.status.success() {
    if !cmd.stdout.is_empty() {
        Ok(cmd.stdout)
    } else {
        error!(
            stderr = String::from_utf8_lossy(&cmd.stderr).to_string(),
            stdout = String::from_utf8_lossy(&cmd.stdout).to_string(),
            "extract attachment failed",
        );
        Err(Error::Ffmpeg)
    }
}

pub async fn extract_stream(path: &Path, index: u64) -> Result<Vec<u8>> {
    let cmd = Command::new("ffmpeg")
        // .args(["-v", "quiet", "-i"])
        .args(["-i"])
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
        .stderr(Stdio::inherit())
        .output()
        .await?;
    if cmd.status.success() {
        Ok(cmd.stdout)
    } else {
        error!(
            stderr = String::from_utf8_lossy(&cmd.stderr).to_string(),
            stdout = String::from_utf8_lossy(&cmd.stdout).to_string(),
            "extract stream failed",
        );
        Err(Error::Ffmpeg)
    }
}

pub async fn generate_thumb(path: &Path) -> Result<Vec<u8>> {
    let cmd = Command::new("ffmpeg")
        // .args(["-v", "quiet", "-i"])
        .args(["-i"])
        .arg(path)
        .args(["-vf", "thumbnail", "-frames:v", "1", "-f", "webp", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .await?;
    if cmd.status.success() {
        Ok(cmd.stdout)
    } else {
        error!(
            stderr = String::from_utf8_lossy(&cmd.stderr).to_string(),
            stdout = String::from_utf8_lossy(&cmd.stdout).to_string(),
            "generate thumb failed",
        );
        Err(Error::Ffmpeg)
    }
}
