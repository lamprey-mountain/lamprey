use std::{
    io::{BufWriter, Error as IoError, Result as IoResult, Write},
    ops::Range,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use opendal::Operator;
use tantivy::{
    directory::{
        error::{DeleteError, OpenReadError, OpenWriteError},
        FileHandle, OwnedBytes, TerminatingWrite, WatchCallback, WatchHandle, WritePtr,
    },
    Directory, HasLen,
};
use tokio::runtime::Runtime;
use tracing::error;

use crate::ServerStateInner;

// TODO: write out all of this

/// minimal shim runtime for tantivy
// TODO: replace with just rt?
struct AsyncIo {
    rt: Runtime,
    // apparently using s.blobs doesnt work
    // s: Arc<ServerStateInner>,
    blobs: opendal::Operator,
}

impl std::fmt::Debug for AsyncIo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AsyncIo {{ ... }}")
    }
}

/// a directory on object storage
#[derive(Debug, Clone)]
pub struct ObjectDirectory {
    io: Arc<AsyncIo>,

    /// which directory to write inside of the object store
    base_path: PathBuf,

    /// location of the local filesystem cache
    cache_path: PathBuf,
    // read_version: Option<u64>,
    // write_version: u64,
    atomic_rw_lock: Arc<Mutex<()>>,
}

/// a file on object storage
#[derive(Debug)]
struct ObjectFile {
    io: Arc<AsyncIo>,
    path: PathBuf,
    len: usize,
}

struct ObjectFileWrite;

impl ObjectDirectory {
    pub fn new(s: Arc<ServerStateInner>, base_path: PathBuf, cache_path: PathBuf) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let blobs: opendal::Operator =
            todo!("somehow clone existing operator here while making it combatible");
        Self {
            io: Arc::new(AsyncIo { rt, blobs }),
            base_path,
            cache_path,
            atomic_rw_lock: Arc::new(Mutex::new(())),
        }
    }
}

impl ObjectDirectory {
    fn blobs(&self) -> &opendal::Operator {
        &self.io.blobs
    }
}

impl Directory for ObjectDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        todo!()
    }

    fn delete(&self, path: &Path) -> Result<(), DeleteError> {
        if let Err(err) = std::fs::remove_file(self.cache_path.join(path)) {
            error!(path = ?path, "failed to remove file from cache");
        }

        let result = self.io.rt.block_on(
            self.blobs()
                .delete(self.base_path.join(path).to_str().unwrap()),
        );

        result.map_err(|err| DeleteError::IoError {
            // TODO: map opendal errors to std::io better
            io_error: Arc::new(IoError::new(std::io::ErrorKind::Other, err)),
            filepath: path.to_path_buf(),
        })
    }

    fn exists(&self, path: &Path) -> Result<bool, OpenReadError> {
        if self.cache_path.join(path).exists() {
            return Ok(true);
        }

        let result = self.io.rt.block_on(
            self.blobs()
                .exists(self.base_path.join(path).to_str().unwrap()),
        );

        result.map_err(|err| OpenReadError::IoError {
            // TODO: map opendal errors to std::io better
            io_error: Arc::new(IoError::new(std::io::ErrorKind::Other, err)),
            filepath: path.to_path_buf(),
        })
    }

    fn open_write(&self, path: &Path) -> Result<WritePtr, OpenWriteError> {
        Ok(BufWriter::new(Box::new(ObjectFileWrite {})))
    }

    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError> {
        let _lock = self.atomic_rw_lock.lock().unwrap();
        let result = self
            .io
            .rt
            .block_on(self.blobs().read(path.to_str().unwrap()));

        result
            .map_err(|err| OpenReadError::IoError {
                // TODO: map opendal errors to std::io better
                io_error: Arc::new(IoError::new(std::io::ErrorKind::Other, err)),
                filepath: path.to_path_buf(),
            })
            .map(|buf| buf.to_vec())
    }

    fn atomic_write(&self, path: &Path, data: &[u8]) -> IoResult<()> {
        let _lock = self.atomic_rw_lock.lock().unwrap();

        let result = self
            .io
            .rt
            .block_on(self.blobs().write(path.to_str().unwrap(), data.to_vec()));

        result
            .map(|_| ())
            .map_err(|err| IoError::new(std::io::ErrorKind::Other, err))
    }

    fn sync_directory(&self) -> IoResult<()> {
        todo!()
    }

    fn watch(&self, watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        todo!()
    }
}

impl FileHandle for ObjectFile {
    fn read_bytes(&self, range: Range<usize>) -> IoResult<OwnedBytes> {
        self.io.rt.block_on(async {});
        todo!()
    }
}

impl HasLen for ObjectFile {
    fn len(&self) -> usize {
        self.len
    }
}

impl TerminatingWrite for ObjectFileWrite {
    fn terminate_ref(&mut self, _: tantivy::directory::AntiCallToken) -> std::io::Result<()> {
        todo!()
    }
}

impl Write for ObjectFileWrite {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        todo!()
    }

    fn flush(&mut self) -> IoResult<()> {
        todo!()
    }
}
