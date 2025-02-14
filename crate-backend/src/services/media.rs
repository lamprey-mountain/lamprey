use std::sync::Arc;

use async_tempfile::TempFile;
use dashmap::DashMap;
use tokio::{io::BufWriter, process::Command};
use tracing::trace;
use types::{MediaCreate, MediaId, UserId};

use crate::{error::Result, ServerStateInner};

mod ffprobe;

#[derive(Debug, Clone, Copy)]
pub struct Metadata {
    pub height: Option<u64>,
    pub width: Option<u64>,
    pub duration: Option<u64>,
}

pub struct ServiceMedia {
    pub state: Arc<ServerStateInner>,
    pub uploads: DashMap<MediaId, MediaUpload>,
}

pub struct MediaUpload {
    pub create: MediaCreate,
    pub user_id: UserId,
    pub temp_file: TempFile,
    pub temp_writer: BufWriter<TempFile>,
}

impl ServiceMedia {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            uploads: DashMap::new(),
        }
    }

    pub async fn create_upload(
        &self,
        media_id: MediaId,
        user_id: UserId,
        create: MediaCreate,
    ) -> Result<()> {
        let temp_file = TempFile::new().await.expect("failed to create temp file!");
        let temp_writer = BufWriter::new(temp_file.open_rw().await?);
        trace!("create temp_file {:?}", temp_file.file_path());
        self.uploads.insert(
            media_id,
            MediaUpload {
                create,
                user_id,
                temp_file,
                temp_writer,
            },
        );
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_metadata_and_mime(
        &self,
        file: &std::path::Path,
    ) -> Result<(Option<Metadata>, String)> {
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
        if !out.status.success() {
            let mime = self.get_mime(file).await?;
            return Ok((None, mime));
        }
        let meta: ffprobe::Metadata = serde_json::from_slice(&out.stdout)?;
        let mut mime = self.get_mime(file).await?;
        // HACK: fix webm
        if meta.is_video() {
            mime = mime.replace("video/webm", "audio/webm");
        }
        Ok((
            Some(Metadata {
                height: meta.width(),
                width: meta.height(),
                duration: meta.duration().map(|i| i as u64),
            }),
            mime,
        ))
    }

    async fn get_mime(&self, file: &std::path::Path) -> Result<String> {
        let out = Command::new("file").arg("-ib").arg(file).output().await?;
        let mime = String::from_utf8(out.stdout).expect("file has failed me");
        Ok(mime)
    }

    // pub async fn generate_thumbnail(
    //     &self,
    //     media_id: MediaId,
    //     path: &Path,
    //     force: bool,
    // ) -> Result<()> {
    //     let data = self.state.data();
    //     let media = data.media_select(media_id).await?;
    //     if media.thumbnail_url.is_some() && !force {
    //         return Ok(());
    //     }
    //     Ok(())
    //     // if let Some() = {}
    //     // media.url
    //     // media_id
    // }
}

// TEMP: copied from an old project
// pub async fn thumbnail(&self, buffer: &[u8]) -> Option<Vec<u8>> {
//     trace!("generate thumbnail");
//     match self
//         .streams
//         .iter()
//         .find(|s| s.disposition.attached_pic == 1)
//     {
//         Some(stream) => ffmpeg::extract(
//             buffer,
//             &["-map", &format!("0:{}", stream.index), "-f", "webp", "-"],
//         )
//         .await
//         .ok(),
//         None => ffmpeg::extract(buffer, &["-vframes", "1", "-f", "webp", "-"])
//             .await
//             .ok(),
//     }
// }

// // FIXME: some files (mp4, mov) may fail to thumbnail with stdin
// // they can have a MOOV atom at the end, and ffmpeg can't seek to the beginning
// pub async fn extract(buffer: &[u8], args: &[&str]) -> Result<Vec<u8>, ()> {
//     let mut cmd = Command::new("ffmpeg")
//         .args([&["-v", "quiet", "-i", "-"], args].concat())
//         .stdin(Stdio::piped())
//         .stdout(Stdio::piped())
//         .stderr(Stdio::inherit())
//         .spawn()
//         .expect("couldn't find ffmpeg");

//     let mut cmd_stdin = cmd.stdin.take().expect("ffmpeg should take stdin");

//     let mut cursor = Cursor::new(&buffer);
//     let _ = tokio::io::copy(&mut cursor, &mut cmd_stdin).await;
//     drop(cmd_stdin);

//     Ok(cmd.wait_with_output().await.map_err(|_| ())?.stdout)
// }
