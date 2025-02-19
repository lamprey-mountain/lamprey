use std::{path::Path, process::Stdio};

use tokio::process::Command;

use crate::Result;

pub async fn extract_attachment(path: &Path, index: u64) -> Result<Vec<u8>> {
    let cmd = Command::new("ffmpeg")
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
        .stderr(Stdio::inherit())
        .output()
        .await?;
    Ok(cmd.stdout)
}

pub async fn generate_thumb(path: &Path) -> Result<Vec<u8>> {
    let cmd = Command::new("ffmpeg")
        .args(["-v", "quiet", "-i"])
        .arg(path)
        .args([
            "-vf",
            "thumbnail,scale=300:300",
            "-frames:v",
            "1",
            "-f",
            "webp",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .await?;
    Ok(cmd.stdout)
}
