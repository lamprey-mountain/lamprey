use async_tempfile::TempFile;
use async_zip::tokio::write::{EntryStreamWriter, ZipFileWriter};
use async_zip::{Compression, ZipEntryBuilder, ZipString};
use futures_util::io::AsyncWriteExt as _;
use serde::Serialize;

use crate::prelude::*;

/// how much ndjson to buffer before writing to the zip
const NDJSON_BUF_SIZE: usize = 64 * 1024;

/// a harvest archive being built in a temporary file
///
/// the archive is compressed, so it is always built in a file and uploaded
/// afterwards, instead of streaming it directly to blob storage.
pub struct Archive {
    file: TempFile,
    zip: Option<ZipFileWriter<TempFile>>,
}

/// an open newline delimited json file inside an [`Archive`]
pub struct Ndjson<'a> {
    writer: EntryStreamWriter<'a, TempFile>,
    buf: Vec<u8>,
}

impl Archive {
    /// create a new harvest archive in a temporary file
    pub async fn create() -> Result<Self> {
        let file = TempFile::new().await?;
        let writer = file.open_rw().await?;

        Ok(Self {
            file,
            zip: Some(ZipFileWriter::with_tokio(writer)),
        })
    }

    /// write a whole file, e.g. `harvest.json` or `readme.txt`
    pub async fn write<T: Serialize + ?Sized>(&mut self, path: &str, value: &T) -> Result<()> {
        let data = serde_json::to_vec(value)?;
        self.zip()
            .write_entry_whole(entry(path), &data)
            .await
            .map_err(zip_error)?;

        Ok(())
    }

    /// write a plain text file
    pub async fn write_text(&mut self, path: &str, text: &str) -> Result<()> {
        self.zip()
            .write_entry_whole(entry(path), text.as_bytes())
            .await
            .map_err(zip_error)?;

        Ok(())
    }

    /// start writing a newline delimited json file
    pub async fn ndjson(&mut self, path: &str) -> Result<Ndjson<'_>> {
        let writer = self
            .zip()
            .write_entry_stream(entry(path))
            .await
            .map_err(zip_error)?;

        Ok(Ndjson {
            writer,
            buf: Vec::with_capacity(NDJSON_BUF_SIZE),
        })
    }

    /// finish the archive, returning the temporary file it was written to
    pub async fn finish(mut self) -> Result<TempFile> {
        let zip = self
            .zip
            .take()
            .ok_or(Error::Internal("archive already finished".into()))?;
        zip.close().await.map_err(zip_error)?;

        Ok(self.file)
    }

    fn zip(&mut self) -> &mut ZipFileWriter<TempFile> {
        self.zip.as_mut().expect("archive is already finished")
    }
}

impl Ndjson<'_> {
    pub async fn line<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        serde_json::to_writer(&mut self.buf, value)?;
        self.buf.push(b'\n');

        if self.buf.len() >= NDJSON_BUF_SIZE {
            self.flush().await?;
        }

        Ok(())
    }

    /// append already serialized ndjson
    pub async fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.buf.extend_from_slice(bytes);

        if self.buf.len() >= NDJSON_BUF_SIZE {
            self.flush().await?;
        }

        Ok(())
    }

    pub async fn finish(mut self) -> Result<()> {
        self.flush().await?;
        self.writer.close().await.map_err(zip_error)?;

        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }

        let mut buf = std::mem::take(&mut self.buf);
        self.writer.write_all(&buf).await?;
        buf.clear();
        self.buf = buf;

        Ok(())
    }
}

fn zip_error(error: async_zip::error::ZipError) -> Error {
    Error::Internal(error.to_string())
}

fn entry(path: &str) -> ZipEntryBuilder {
    ZipEntryBuilder::new(ZipString::from(path), Compression::Deflate)
}
