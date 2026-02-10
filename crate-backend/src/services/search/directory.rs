use std::{
    io::{BufWriter, Error as IoError, Result as IoResult, Write},
    ops::Range,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use opendal::{layers::LoggingLayer, Builder, Operator};
use tantivy::{
    directory::{
        error::{DeleteError, OpenReadError, OpenWriteError},
        FileHandle, OwnedBytes, TerminatingWrite, WatchCallback, WatchHandle, WritePtr,
    },
    Directory, HasLen,
};
use tokio::runtime::Handle;
use tracing::error;

use crate::ServerStateInner;

/// a directory on object storage using blocking IO
#[derive(Debug, Clone)]
pub struct ObjectDirectory {
    /// opendal operator to access s3
    blobs: Operator,

    /// tokio runtime to use the opendal operator with
    rt: Arc<tokio::runtime::Runtime>,

    /// which directory to write inside of the object store
    base_path: PathBuf,

    /// location of the local filesystem cache
    cache_path: PathBuf,
    atomic_rw_lock: Arc<Mutex<()>>,
}

/// a file on object storage
#[derive(Debug)]
struct ObjectFile {
    rt: Arc<tokio::runtime::Runtime>,
    blobs: Operator,
    path: String,
    len: usize,
}

/// a handle to write to a file on object storage
struct ObjectFileWrite {
    rt: Arc<tokio::runtime::Runtime>,
    blobs: Operator,
    path: String,
    buf: Vec<u8>,
}

impl ObjectDirectory {
    pub fn new(s: Arc<ServerStateInner>, base_path: PathBuf, cache_path: PathBuf) -> Self {
        let config = &s.config;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        // copied from main
        let blobs_builder = opendal::services::S3::default()
            .bucket(&config.s3.bucket)
            .endpoint(config.s3.endpoint.as_str())
            .region(&config.s3.region)
            .access_key_id(&config.s3.access_key_id)
            .secret_access_key(&config.s3.secret_access_key);
        let blobs = Operator::new(blobs_builder)
            .expect("if this worked in main server state it should work here too")
            .layer(LoggingLayer::default())
            .finish();

        Self {
            blobs,
            rt: Arc::new(rt),
            base_path,
            cache_path,
            atomic_rw_lock: Arc::new(Mutex::new(())),
        }
    }

    fn path_str(&self, path: &Path) -> String {
        self.base_path.join(path).to_str().unwrap().to_string()
    }
}

impl Directory for ObjectDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        let p = self.path_str(path);
        let meta = self
            .rt
            .block_on(self.blobs.stat(&p))
            .map_err(|err| OpenReadError::IoError {
                io_error: Arc::new(IoError::new(std::io::ErrorKind::Other, err)),
                filepath: path.to_path_buf(),
            })?;

        Ok(Arc::new(ObjectFile {
            rt: Arc::clone(&self.rt),
            blobs: self.blobs.clone(),
            path: p,
            len: meta.content_length() as usize,
        }))
    }

    fn delete(&self, path: &Path) -> Result<(), DeleteError> {
        if let Err(err) = std::fs::remove_file(self.cache_path.join(path)) {
            if err.kind() != std::io::ErrorKind::NotFound {
                error!(path = ?path, "failed to remove file from cache");
            }
        }

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

        self.rt
            .block_on(self.blobs.exists(&self.path_str(path)))
            .map_err(|err| OpenReadError::IoError {
                io_error: Arc::new(IoError::new(std::io::ErrorKind::Other, err)),
                filepath: path.to_path_buf(),
            })
    }

    fn open_write(&self, path: &Path) -> Result<WritePtr, OpenWriteError> {
        Ok(BufWriter::new(Box::new(ObjectFileWrite {
            rt: Arc::clone(&self.rt),
            blobs: self.blobs.clone(),
            path: self.path_str(path),
            buf: Vec::new(),
        })))
    }

    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError> {
        let _lock = self.atomic_rw_lock.lock().unwrap();
        self.rt
            .block_on(self.blobs.read(&self.path_str(path)))
            .map_err(|err| OpenReadError::IoError {
                io_error: Arc::new(IoError::new(std::io::ErrorKind::Other, err)),
                filepath: path.to_path_buf(),
            })
            .map(|buf| buf.to_vec())
    }

    fn atomic_write(&self, path: &Path, data: &[u8]) -> IoResult<()> {
        let _lock = self.atomic_rw_lock.lock().unwrap();
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

impl FileHandle for ObjectFile {
    fn read_bytes(&self, range: Range<usize>) -> IoResult<OwnedBytes> {
        let range_len = range.end - range.start;
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

impl TerminatingWrite for ObjectFileWrite {
    fn terminate_ref(&mut self, _: tantivy::directory::AntiCallToken) -> std::io::Result<()> {
        self.rt
            .block_on(self.blobs.write(&self.path, self.buf.clone()))
            .map_err(|err| IoError::new(std::io::ErrorKind::Other, err))?;
        Ok(())
    }
}

impl Write for ObjectFileWrite {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.buf.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}
