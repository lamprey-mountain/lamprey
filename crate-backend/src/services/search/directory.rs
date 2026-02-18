use std::{
    io::{BufWriter, Error as IoError, Result as IoResult, Write},
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use dashmap::DashMap;
use opendal::{ErrorKind, Operator};
use tantivy::{
    directory::{
        error::{DeleteError, OpenReadError, OpenWriteError},
        FileHandle, OwnedBytes, TerminatingWrite, WatchCallback, WatchHandle, WritePtr,
    },
    Directory, HasLen,
};
use tokio::runtime::Handle as TokioHandle;
use tracing::debug;

use crate::ServerStateInner;

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
}

/// a handle to write to a file on object storage and local cache
struct ObjectFileWrite {
    file: std::fs::File,
    rt: TokioHandle,
    writer: opendal::Writer,
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
        self.base_path.join(path).to_str().unwrap().to_string()
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
        if cache_file.exists() {
            let meta = std::fs::metadata(&cache_file).map_err(|err| OpenReadError::IoError {
                io_error: Arc::new(err),
                filepath: path.to_path_buf(),
            })?;
            return Ok(Arc::new(ObjectFile {
                rt: self.rt.clone(),
                blobs: self.blobs.clone(),
                path: self.path_str(path),
                len: meta.len() as usize,
                cache_path: cache_file,
            }));
        }

        debug!(path = ?path, "downloading file from object store to cache");
        let metadata = self.metadata(path)?;

        Ok(Arc::new(ObjectFile {
            rt: self.rt.clone(),
            blobs: self.blobs.clone(),
            path: self.path_str(path),
            len: metadata.len,
            cache_path: cache_file,
        }))
    }

    fn delete(&self, path: &Path) -> Result<(), DeleteError> {
        // delete local file
        let cache_file = self.cache_path.join(path);
        if cache_file.exists() {
            std::fs::remove_file(&cache_file).map_err(|err| DeleteError::IoError {
                io_error: Arc::new(err),
                filepath: path.to_path_buf(),
            })?;
        }

        // delete the remote file even if the file isn't found locally
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
        let writer = self
            .rt
            .block_on(self.blobs.writer(&self.path_str(path)))
            .map_err(|err| OpenWriteError::IoError {
                io_error: Arc::new(IoError::new(std::io::ErrorKind::Other, err)),
                filepath: path.to_path_buf(),
            })?;

        let cache_file_path = self.cache_path.join(path);
        if let Some(parent) = cache_file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| OpenWriteError::IoError {
                io_error: Arc::new(err),
                filepath: path.to_path_buf(),
            })?;
        }
        let file = std::fs::File::create(&cache_file_path).map_err(|err| OpenWriteError::IoError {
            io_error: Arc::new(err),
            filepath: path.to_path_buf(),
        })?;

        Ok(BufWriter::new(Box::new(ObjectFileWrite {
            file,
            rt: self.rt.clone(),
            writer,
        })))
    }

    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError> {
        if self.cache_path.join(path).exists() {
            return std::fs::read(self.cache_path.join(path)).map_err(|err| {
                OpenReadError::IoError {
                    io_error: Arc::new(err),
                    filepath: path.to_path_buf(),
                }
            });
        }

        self.rt
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
            })
            .map(|buf| buf.to_vec())
    }

    fn atomic_write(&self, path: &Path, data: &[u8]) -> IoResult<()> {
        // write locally
        std::fs::write(self.cache_path.join(path), data)?;

        // write remotely
        self.rt
            .block_on(self.blobs.write(&self.path_str(path), data.to_vec()))
            .map_err(|err| IoError::new(std::io::ErrorKind::Other, err))
            .map(|_| ())
    }

    fn sync_directory(&self) -> IoResult<()> {
        Ok(())
    }

    fn watch(&self, _watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        Ok(WatchHandle::empty())
    }
}

const SMALL_FILE: usize = 512;

impl FileHandle for ObjectFile {
    fn read_bytes(&self, range: Range<usize>) -> IoResult<OwnedBytes> {
        let range_len = range.end - range.start;

        // if the file is small enough, download the whole file and cache it
        if self.len < SMALL_FILE {
            debug!(path = ?self.path, len = self.len, "downloading small file to cache");
            let buf = self
                .rt
                .block_on(async {
                    self.blobs
                        .read_with(&self.path)
                        .await
                })
                .map_err(|err| IoError::new(std::io::ErrorKind::Other, err))?;

            // convert buffer to bytes
            let buf = buf.to_vec();

            // write to cache
            if let Some(parent) = self.cache_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&self.cache_path, &buf)?;

            if buf.len() != self.len {
                return Err(IoError::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "failed to read full file from object store",
                ));
            }

            return Ok(OwnedBytes::new(buf[range.start..range.end].to_vec()));
        }

        let buf = self
            .rt
            .block_on(async {
                self.blobs
                    .read_with(&self.path)
                    .range(range.start as u64..range.end as u64)
                    .await
            })
            .map_err(|err| IoError::new(std::io::ErrorKind::Other, err))?;

        if buf.len() != range_len {
            return Err(IoError::new(
                std::io::ErrorKind::UnexpectedEof,
                "failed to read full range from object store",
            ));
        }

        Ok(OwnedBytes::new(buf.to_vec()))
    }
}

impl HasLen for ObjectFile {
    fn len(&self) -> usize {
        self.len
    }
}

impl Write for ObjectFileWrite {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let len = buf.len();
        self.file.write_all(buf)?;
        self.rt
            .block_on(self.writer.write(buf.to_vec()))
            .map_err(|err| IoError::new(std::io::ErrorKind::Other, err))?;
        Ok(len)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.file.flush()?;
        Ok(())
    }
}

impl TerminatingWrite for ObjectFileWrite {
    fn terminate_ref(&mut self, _: tantivy::directory::AntiCallToken) -> std::io::Result<()> {
        self.file.flush()?;
        self.rt
            .block_on(self.writer.close())
            .map_err(|err| IoError::new(std::io::ErrorKind::Other, err))?;
        Ok(())
    }
}
