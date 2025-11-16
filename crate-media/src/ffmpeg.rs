use std::{path::Path, process::Stdio};

use tokio::process::Command;
use tracing::error;

use crate::{error::Error, Result};

pub async fn transcode_to_webm(in_path: &Path, out_path: &Path) -> Result<()> {
    let cmd = Command::new("ffmpeg")
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
        error!(
            stderr = String::from_utf8_lossy(&cmd.stderr).to_string(),
            stdout = String::from_utf8_lossy(&cmd.stdout).to_string(),
            "transcode failed",
        );
        Err(Error::Ffmpeg)
    }
}
