use std::io::{Error as IoError, Read, Result as IoResult, Seek, SeekFrom, Write};
use std::ops::Range;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use once_cell::sync::OnceCell;
use opendal::{ErrorKind, Operator};
use tantivy::{
    directory::{
        error::{DeleteError, OpenReadError, OpenWriteError},
        FileHandle, OwnedBytes, TerminatingWrite, WatchCallback, WatchHandle, WritePtr,
    },
    Directory, HasLen,
};
use tokio::runtime::Handle as TokioHandle;
use tokio::task::JoinSet;
use tracing::{debug, warn};

use crate::ServerStateInner;

/// block size for chunked caching (1MB)
const BLOCK_SIZE: usize = 1024 * 1024;

/// file extensions that are considered "hot" and should be eagerly downloaded in full
const HOT_EXTENSIONS: &[&str] = &["term", "json", "idx", "del", "fieldnorm"];

fn is_hot_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| HOT_EXTENSIONS.contains(&ext))
}

/// a directory on object storage using a local filesystem cache
#[derive(Debug, Clone)]
pub struct ObjectDirectory {
    /// opendal operator to access s3
    blobs: Operator,

    /// tokio runtime to use the opendal operator with
    rt: TokioHandle,

    /// which directory to write inside of the object store
    base_path: PathBuf,

    /// location of the local filesystem cache
    cache_path: PathBuf,

    /// cache of object metadata
    cache_metadata: Arc<DashMap<PathBuf, ObjectMetadata>>,
}

/// metadata for a file
#[derive(Debug, Clone)]
struct ObjectMetadata {
    /// the length of the file
    len: usize,
}

/// a file on object storage
#[derive(Debug)]
struct ObjectFile {
    rt: TokioHandle,
    blobs: Operator,
    path: String,
    len: usize,
    cache_path: PathBuf,
    fully_cached: bool,
    handle: OnceCell<std::fs::File>,
}

struct ObjectFileWrite {
    file: Option<std::fs::File>,
    rt: TokioHandle,
    blobs: Operator,
    remote_path: String,
    path: PathBuf,
    cache_file_path: PathBuf,
    temp_path: PathBuf,
    cache_metadata: Arc<DashMap<PathBuf, ObjectMetadata>>,
    total_len: usize,
}

impl ObjectDirectory {
    pub fn new(s: Arc<ServerStateInner>, base_path: PathBuf, cache_path: PathBuf) -> Self {
        std::fs::create_dir_all(&cache_path).expect("failed to create cache directory");
        Self {
            blobs: s.blobs.clone(),
            rt: s.tokio.clone(),
            base_path,
            cache_path,
            cache_metadata: Arc::new(DashMap::new()),
        }
    }

    fn path_str(&self, path: &Path) -> String {
        let relative = path.strip_prefix("/").unwrap_or(path);
        self.base_path.join(relative).to_str().unwrap().to_string()
    }

    /// get metadata for an object
    fn metadata(&self, path: &Path) -> Result<ObjectMetadata, OpenReadError> {
        if let Some(meta) = self.cache_metadata.get(path) {
            return Ok(meta.clone());
        }

        let p = self.path_str(path);
        let meta = self.rt.block_on(self.blobs.stat(&p)).map_err(|err| {
            if err.kind() == ErrorKind::NotFound {
                OpenReadError::FileDoesNotExist(path.to_path_buf())
            } else {
                OpenReadError::IoError {
                    io_error: Arc::new(IoError::new(std::io::ErrorKind::Other, err)),
                    filepath: path.to_path_buf(),
                }
            }
        })?;

        let metadata = ObjectMetadata {
            len: meta.content_length() as usize,
        };
        self.cache_metadata
            .insert(path.to_path_buf(), metadata.clone());
        Ok(metadata)
    }
}

impl Directory for ObjectDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        let cache_file = self.cache_path.join(path);
        let metadata = self.metadata(path)?;

        let handle = OnceCell::new();
        if cache_file.exists() {
            if let Ok(f) = std::fs::File::open(&cache_file) {
                let _ = handle.set(f);
            }
        }

        Ok(Arc::new(ObjectFile {
            rt: self.rt.clone(),
            blobs: self.blobs.clone(),
            path: self.path_str(path),
            len: metadata.len,
            cache_path: cache_file.clone(),
            fully_cached: cache_file.exists(),
            handle,
        }))
    }

    fn delete(&self, path: &Path) -> Result<(), DeleteError> {
        let cache_file = self.cache_path.join(path);
        let chunk_prefix = format!(
            "{}.",
            cache_file
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
        );

        if cache_file.exists() {
            let _ = std::fs::remove_file(&cache_file);
        }

        // scan the actual parent directory of the file, not the root cache dir,
        // so chunks in subdirectories are found and deleted
        if let Some(parent) = cache_file.parent() {
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    let name_str = entry.file_name().to_string_lossy().into_owned();
                    if name_str.starts_with(&chunk_prefix) && name_str.ends_with(".chunk") {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }

        self.cache_metadata.remove(path);
        self.rt
            .block_on(self.blobs.delete(&self.path_str(path)))
            .map_err(|err| DeleteError::IoError {
                io_error: Arc::new(IoError::new(std::io::ErrorKind::Other, err)),
                filepath: path.to_path_buf(),
            })
    }

    fn exists(&self, path: &Path) -> Result<bool, OpenReadError> {
        if self.cache_path.join(path).exists() {
            return Ok(true);
        }

        match self.rt.block_on(self.blobs.exists(&self.path_str(path))) {
            Ok(exists) => Ok(exists),
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(OpenReadError::IoError {
                        io_error: Arc::new(IoError::new(std::io::ErrorKind::Other, err)),
                        filepath: path.to_path_buf(),
                    })
                }
            }
        }
    }

    fn open_write(&self, path: &Path) -> Result<WritePtr, OpenWriteError> {
        let cache_file_path = self.cache_path.join(path);
        if let Some(parent) = cache_file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| OpenWriteError::IoError {
                io_error: Arc::new(err),
                filepath: path.to_path_buf(),
            })?;
        }

        let temp_path = cache_file_path.with_file_name(format!(
            "{}.{}.tmp",
            cache_file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(""),
            uuid::Uuid::new_v4()
        ));

        let file = std::fs::File::create(&temp_path).map_err(|err| OpenWriteError::IoError {
            io_error: Arc::new(err),
            filepath: path.to_path_buf(),
        })?;

        Ok(WritePtr::new(Box::new(ObjectFileWrite {
            file: Some(file),
            rt: self.rt.clone(),
            blobs: self.blobs.clone(),
            remote_path: self.path_str(path),
            path: path.to_path_buf(),
            cache_file_path,
            temp_path,
            cache_metadata: self.cache_metadata.clone(),
            total_len: 0,
        })))
    }

    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError> {
        let full_cache_path = self.cache_path.join(path);
        if full_cache_path.exists() {
            return std::fs::read(&full_cache_path).map_err(|err| OpenReadError::IoError {
                io_error: Arc::new(err),
                filepath: path.to_path_buf(),
            });
        }

        let buf = self
            .rt
            .block_on(self.blobs.read(&self.path_str(path)))
            .map_err(|err| {
                if err.kind() == ErrorKind::NotFound {
                    OpenReadError::FileDoesNotExist(path.to_path_buf())
                } else {
                    OpenReadError::IoError {
                        io_error: Arc::new(IoError::new(std::io::ErrorKind::Other, err)),
                        filepath: path.to_path_buf(),
                    }
                }
            })?
            .to_vec();

        // atomic write: temp file + rename, with parent dir creation
        if let Some(parent) = full_cache_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| OpenReadError::IoError {
                io_error: Arc::new(err),
                filepath: path.to_path_buf(),
            })?;
        }
        let tmp_path = full_cache_path.with_extension("tmp");
        std::fs::write(&tmp_path, &buf).map_err(|err| OpenReadError::IoError {
            io_error: Arc::new(err),
            filepath: path.to_path_buf(),
        })?;
        std::fs::rename(&tmp_path, &full_cache_path).map_err(|err| OpenReadError::IoError {
            io_error: Arc::new(err),
            filepath: path.to_path_buf(),
        })?;

        Ok(buf)
    }

    fn atomic_write(&self, path: &Path, data: &[u8]) -> IoResult<()> {
        let cache_file_path = self.cache_path.join(path);

        // write to temp file first, then rename for atomicity
        if let Some(parent) = cache_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp_path = cache_file_path.with_extension("tmp");
        std::fs::write(&tmp_path, data)?;
        std::fs::rename(&tmp_path, &cache_file_path)?;

        // write remotely
        self.rt
            .block_on(self.blobs.write(&self.path_str(path), data.to_vec()))
            .map_err(|err| IoError::new(std::io::ErrorKind::Other, err))?;

        self.cache_metadata
            .insert(path.to_path_buf(), ObjectMetadata { len: data.len() });
        Ok(())
    }

    fn sync_directory(&self) -> IoResult<()> {
        Ok(())
    }

    fn watch(&self, _watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        Ok(WatchHandle::empty())
    }
}

impl FileHandle for ObjectFile {
    fn read_bytes(&self, range: Range<usize>) -> IoResult<OwnedBytes> {
        // if the file is already fully cached, read directly from disk
        if self.fully_cached || self.cache_path.exists() {
            let mut file = self.handle.get_or_try_init(|| std::fs::File::open(&self.cache_path))?;
            let mut buf = vec![0u8; range.end - range.start];
            file.read_exact_at(&mut buf, range.start as u64)?;
            return Ok(OwnedBytes::new(buf));
        }

        // for hot files, download the entire file once and cache it
        if is_hot_file(&self.cache_path) {
            // check if another request already cached this file
            if !self.cache_path.exists() {
                debug!(path = ?self.path, len = self.len, "downloading hot file to cache");
                let buf = self
                    .rt
                    .block_on(async { self.blobs.read(&self.path).await })
                    .map_err(|err| IoError::new(std::io::ErrorKind::Other, err))?
                    .to_vec();

                // atomic write with parent dir creation
                if let Some(parent) = self.cache_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let tmp_path = self.cache_path.with_extension("tmp");
                std::fs::write(&tmp_path, &buf)?;
                std::fs::rename(&tmp_path, &self.cache_path)?;
            }

            // fall through to read from the now-cached file
            let mut file = std::fs::File::open(&self.cache_path)?;
            file.seek(SeekFrom::Start(range.start as u64))?;
            let mut buf = vec![0u8; range.end - range.start];
            file.read_exact(&mut buf)?;
            return Ok(OwnedBytes::new(buf));
        }

        // for all other files, use block-based caching
        self.read_bytes_blocked(range)
    }
}

impl ObjectFile {
    /// read bytes using block-based caching. downloads only the 1MB blocks that
    /// overlap the requested range and persists them to disk for future reads.
    fn read_bytes_blocked(&self, range: Range<usize>) -> IoResult<OwnedBytes> {
        if range.start >= range.end {
            return Ok(OwnedBytes::empty());
        }

        let start_block = range.start / BLOCK_SIZE;
        let end_block = (range.end - 1) / BLOCK_SIZE;

        // 1. identify missing blocks and download them in parallel
        let mut missing_blocks = Vec::new();
        for idx in start_block..=end_block {
            if !self.block_path(idx).exists() {
                missing_blocks.push(idx);
            }
        }

        if !missing_blocks.is_empty() {
            self.rt.block_on(async {
                let mut set = JoinSet::new();
                for idx in missing_blocks {
                    let blobs = self.blobs.clone();
                    let path = self.path.clone();
                    let block_path = self.block_path(idx);
                    let file_len = self.len;

                    set.spawn(async move {
                        let b_start = idx * BLOCK_SIZE;
                        let b_end = ((idx + 1) * BLOCK_SIZE).min(file_len);
                        let buf = blobs
                            .read_with(&path)
                            .range(b_start as u64..b_end as u64)
                            .await?;

                        // create parent dirs and use uuid-named temp file to avoid
                        // concurrent rename races
                        if let Some(parent) = block_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let tmp_path =
                            block_path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
                        std::fs::write(&tmp_path, buf.to_vec())?;
                        std::fs::rename(&tmp_path, &block_path)?;

                        Ok::<(), IoError>(())
                    });
                }

                while let Some(res) = set.join_next().await {
                    if let Err(e) = res {
                        warn!("Block download task failed: {e}");
                    }
                }
            });
        }

        // 2. read required ranges from (now existing) block files
        let mut out_buf = vec![0u8; range.end - range.start];
        let mut bytes_written = 0;

        for idx in start_block..=end_block {
            let block_data = std::fs::read(self.block_path(idx))?;
            let b_start_offset = idx * BLOCK_SIZE;
            let read_start = range.start.max(b_start_offset) - b_start_offset;
            let read_end = range
                .end
                .min((idx + 1) * BLOCK_SIZE)
                .min(b_start_offset + block_data.len())
                - b_start_offset;
            let len = read_end.saturating_sub(read_start);

            out_buf[bytes_written..bytes_written + len]
                .copy_from_slice(&block_data[read_start..read_end]);
            bytes_written += len;
        }

        Ok(OwnedBytes::new(out_buf))
    }

    /// get the path for a specific block of this file
    fn block_path(&self, block_idx: usize) -> PathBuf {
        let mut p = self.cache_path.clone();
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("data");
        p.set_extension(format!("{block_idx}.{ext}.chunk"));
        p
    }
}

impl HasLen for ObjectFile {
    fn len(&self) -> usize {
        self.len
    }
}

impl Write for ObjectFileWrite {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        if let Some(file) = &mut self.file {
            file.write_all(buf)?;
        }
        self.total_len += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> IoResult<()> {
        if let Some(file) = &mut self.file {
            file.flush()?;
        }
        Ok(())
    }
}

impl TerminatingWrite for ObjectFileWrite {
    fn terminate_ref(&mut self, _: tantivy::directory::AntiCallToken) -> std::io::Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
        }

        // streaming upload to prevent oom
        let mut file = std::fs::File::open(&self.temp_path)?;
        self.rt
            .block_on(async {
                let mut writer = self.blobs.writer(&self.remote_path).await?;
                let mut buf = vec![0u8; 8 * 1024 * 1024]; // 8MB upload buffer
                loop {
                    let n = file.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    writer.write(buf[..n].to_vec()).await?;
                }
                writer.close().await?;
                Ok::<(), IoError>(())
            })
            .map_err(|e| IoError::new(std::io::ErrorKind::Other, e))?;

        // rename temp file to cache file
        std::fs::rename(&self.temp_path, &self.cache_file_path)?;

        self.cache_metadata.insert(
            self.path.clone(),
            ObjectMetadata {
                len: self.total_len,
            },
        );
        Ok(())
    }
}
