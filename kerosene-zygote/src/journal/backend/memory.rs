//! A [`MemoryBackend`] stores data in memory.

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{
    Stream, StreamExt,
    channel::oneshot,
    stream::{self, BoxStream},
};

use super::{Backend, BlockStream, BoxFuture};
use crate::journal::{
    Block, Config, JournalId, Metadata, OpenError, ReadError, WriteError, meta::JournalEvent,
};

#[cfg(test)]
mod tests;

/// A [`Backend`] that stores journals in memory.
#[derive(Clone)]
pub struct MemoryBackend {
    inner: Arc<RwLock<Inner>>,
}

#[derive(Default)]
struct Inner {
    journals: HashMap<JournalId, JournalState>,
}

struct JournalState {
    config: Config,
    blocks: Vec<Bytes>,
    size: u64,
    waiters: Vec<oneshot::Sender<()>>,
}

struct MemoryStream {
    backend: MemoryBackend,
    id: JournalId,
    next: u64,
    /// The end of a finite read's snapshot. `None` for tailing reads.
    end: Option<u64>,
    tail: bool,
    done: bool,
    wait: Option<oneshot::Receiver<()>>,
}

enum Outcome {
    Block(u64, Bytes),
    Wait(oneshot::Receiver<()>),
    End,
    Deleted,
}

impl MemoryBackend {
    /// Create an empty in-memory backend.
    pub fn new() -> Self {
        let mut inner = Inner::default();
        inner
            .journals
            .insert(JournalId::META, JournalState::new(Config::default()));

        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    fn read_inner(&self) -> RwLockReadGuard<'_, Inner> {
        self.inner.read().expect("memory backend poisoned")
    }

    fn write_inner(&self) -> RwLockWriteGuard<'_, Inner> {
        self.inner.write().expect("memory backend poisoned")
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for MemoryBackend {
    fn create(&self, config: Config) -> BoxFuture<'_, Result<(JournalId, Metadata), OpenError>> {
        Box::pin(async move {
            let mut inner = self.write_inner();
            let id = inner.new_id();
            let state = JournalState::new(config);
            let metadata = state.metadata();

            inner.journals.insert(id, state);
            inner.append_meta(JournalEvent::Create(id));

            Ok((id, metadata))
        })
    }

    fn open(&self, id: JournalId) -> BoxFuture<'_, Result<Metadata, OpenError>> {
        Box::pin(async move {
            self.read_inner()
                .journals
                .get(&id)
                .map(JournalState::metadata)
                .ok_or(OpenError::Missing)
        })
    }

    fn delete(&self, id: JournalId) -> BoxFuture<'_, Result<(), WriteError>> {
        Box::pin(async move {
            let mut inner = self.write_inner();

            if id.is_meta() {
                if let Some(mut state) = inner.journals.remove(&id) {
                    state.notify();
                }
                return Ok(());
            }

            let Some(mut state) = inner.journals.remove(&id) else {
                return Err(WriteError::Deleted);
            };
            state.notify();
            inner.append_meta(JournalEvent::Delete(id));

            Ok(())
        })
    }

    fn list(&self) -> BoxStream<'static, Result<(JournalId, Metadata), ReadError>> {
        let inner = self.read_inner();
        let journals: Vec<_> = inner
            .journals
            .iter()
            .filter(|(id, _)| !id.is_meta())
            .map(|(id, state)| (*id, state.metadata()))
            .collect();

        stream::iter(journals.into_iter().map(Ok)).boxed()
    }

    fn append(&self, id: JournalId, block: Bytes) -> BoxFuture<'_, Result<u64, WriteError>> {
        Box::pin(async move {
            if id.is_meta() {
                return Err(WriteError::MetaWrite);
            }

            let mut inner = self.write_inner();
            let state = inner.journals.get_mut(&id).ok_or(WriteError::Deleted)?;
            if state.config.frozen {
                return Err(WriteError::Frozen);
            }
            if block.len() as u64 > state.config.max_block_len {
                return Err(WriteError::Oversized);
            }

            Ok(state.append(block))
        })
    }

    fn sync(&self, id: JournalId) -> BoxFuture<'_, Result<(), WriteError>> {
        Box::pin(async move {
            if self.read_inner().journals.contains_key(&id) {
                Ok(())
            } else {
                Err(WriteError::Deleted)
            }
        })
    }

    fn freeze(&self, id: JournalId) -> BoxFuture<'_, Result<bool, WriteError>> {
        Box::pin(async move {
            if id.is_meta() {
                return Err(WriteError::MetaFreeze);
            }

            let mut inner = self.write_inner();
            let state = inner.journals.get_mut(&id).ok_or(WriteError::Deleted)?;
            if state.config.frozen {
                return Ok(false);
            }
            state.config.frozen = true;
            // no more blocks will ever be appended, so let tailing readers finish
            state.notify();

            inner.append_meta(JournalEvent::Freeze(id));

            Ok(true)
        })
    }

    fn stat(&self, id: JournalId) -> BoxFuture<'_, Result<Metadata, ReadError>> {
        Box::pin(async move {
            self.read_inner()
                .journals
                .get(&id)
                .map(JournalState::metadata)
                .ok_or(ReadError::Deleted)
        })
    }

    fn read(
        &self,
        id: JournalId,
        start: u64,
        tail: bool,
    ) -> BoxFuture<'_, Result<BlockStream, ReadError>> {
        Box::pin(async move {
            let end = {
                let inner = self.read_inner();
                let state = inner.journals.get(&id).ok_or(ReadError::Deleted)?;
                (!tail).then(|| state.end())
            };

            Ok(MemoryStream {
                backend: self.clone(),
                id,
                next: start,
                end,
                tail,
                done: false,
                wait: None,
            }
            .boxed())
        })
    }
}

impl Inner {
    fn new_id(&self) -> JournalId {
        loop {
            let id = JournalId(rand::random());
            if !id.is_meta() && !self.journals.contains_key(&id) {
                return id;
            }
        }
    }

    /// Append a catalog event, recreating the meta journal if necessary.
    fn append_meta(&mut self, event: JournalEvent) -> u64 {
        self.journals
            .entry(JournalId::META)
            .or_insert_with(|| JournalState::new(Config::default()))
            .append(event.encode())
    }
}

impl JournalState {
    fn new(config: Config) -> Self {
        Self {
            config,
            blocks: Vec::new(),
            size: 0,
            waiters: Vec::new(),
        }
    }

    fn end(&self) -> u64 {
        self.blocks.len() as u64
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            config: self.config.clone(),
            end: self.end(),
            size: self.size,
            physical_size: self.size,
            frozen: self.config.frozen,
        }
    }

    fn append(&mut self, block: Bytes) -> u64 {
        let seq = self.end();
        self.size += block.len() as u64;
        self.blocks.push(block);
        self.notify();
        seq
    }

    fn notify(&mut self) {
        for waiter in self.waiters.drain(..) {
            let _ = waiter.send(());
        }
    }
}

impl Stream for MemoryStream {
    type Item = Result<Block, ReadError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if this.done {
            return Poll::Ready(None);
        }

        loop {
            if let Some(wait) = this.wait.as_mut() {
                match Pin::new(wait).poll(cx) {
                    Poll::Ready(_) => this.wait = None,
                    Poll::Pending => return Poll::Pending,
                }
            }

            // finish finite reads without consulting the journal, so deleting
            // it after the snapshot is exhausted still ends the stream cleanly
            if this.end.is_some_and(|end| this.next >= end) {
                this.done = true;
                return Poll::Ready(None);
            }

            let outcome = {
                let mut inner = this.backend.write_inner();
                match inner.journals.get_mut(&this.id) {
                    None => Outcome::Deleted,
                    Some(state) => {
                        if this.next < state.end() {
                            let data = state.blocks[this.next as usize].clone();
                            Outcome::Block(this.next, data)
                        } else if !this.tail || state.config.frozen {
                            Outcome::End
                        } else {
                            let (tx, rx) = oneshot::channel();
                            state.waiters.push(tx);
                            Outcome::Wait(rx)
                        }
                    }
                }
            };

            match outcome {
                Outcome::Block(seq, data) => {
                    this.next = seq + 1;
                    return Poll::Ready(Some(Ok(Block { seq, data })));
                }
                Outcome::Wait(rx) => this.wait = Some(rx),
                Outcome::End => {
                    this.done = true;
                    return Poll::Ready(None);
                }
                Outcome::Deleted => {
                    this.done = true;
                    return Poll::Ready(Some(Err(ReadError::Deleted)));
                }
            }
        }
    }
}
