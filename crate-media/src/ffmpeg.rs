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

pub async fn generate_thumbnail(
    in_path: &Path,
    out_path: &Path,
    size: u32,
    animate: bool,
) -> Result<()> {
    let mut cmd = Command::new("ffmpeg");
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
        // Generate static AVIF (first frame)
        cmd.args([
            "-vf",
            &format!("scale={size}:{size}:force_original_aspect_ratio=decrease"),
            "-frames:v",
            "1",
            "-f",
            "avif",
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
        Err(Error::Ffmpeg)
    }
}
