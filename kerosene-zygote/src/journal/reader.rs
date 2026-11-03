//! Reading blocks from a [`Journal`].

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Stream, StreamExt};

use super::{Block, Journal, JournalId, ReadError, backend::BlockStream};

/// A cursor over a [`Journal`].
///
/// A reader is created with [`Journal::read`] or [`Journal::tail`]. Readers are
/// cheap handles: many can read the same journal concurrently without
/// interfering with writers.
// TODO: impl AsyncRead, buffering, reading mid-block
pub struct Reader {
    journal: Journal,
    start: u64,
    next: u64,
    end: Option<u64>,
    tail: bool,
    stream: BlockStream,
}

impl Reader {
    pub(crate) fn new(
        journal: Journal,
        start: u64,
        end: Option<u64>,
        tail: bool,
        stream: BlockStream,
    ) -> Self {
        Self {
            journal,
            start,
            next: start,
            end,
            tail,
            stream,
        }
    }

    /// Get the id of the journal this reader is reading.
    pub fn journal_id(&self) -> JournalId {
        self.journal.id()
    }

    /// Get the sequence number this reader was opened at.
    ///
    /// This is the sequence number of the first block the reader returned.
    pub fn start(&self) -> u64 {
        self.start
    }

    /// Get the sequence number of the next block this reader will return.
    pub fn next_seq(&self) -> u64 {
        self.next
    }

    /// Get one past the last block in this reader's snapshot.
    ///
    /// Returns `None` for a tailing reader, whose end moves as blocks are
    /// written.
    pub fn end(&self) -> Option<u64> {
        self.end
    }

    /// Get how many blocks this reader has left in its snapshot.
    ///
    /// Returns `None` for a tailing reader.
    pub fn len(&self) -> Option<u64> {
        self.end.map(|end| end.saturating_sub(self.next))
    }

    /// Returns `true` if this reader has no blocks left in its snapshot.
    ///
    /// Returns `None` for a tailing reader.
    pub fn is_empty(&self) -> Option<bool> {
        self.len().map(|len| len == 0)
    }

    /// Returns `true` if this reader follows new blocks as they are written.
    pub fn is_tail(&self) -> bool {
        self.tail
    }

    /// Read the next block.
    ///
    /// Returns `None` when a finite reader reaches the end of the journal. A
    /// tailing reader waits for the next block instead.
    pub async fn next(&mut self) -> Result<Option<Block>, ReadError> {
        let block = self.stream.next().await.transpose()?;
        if block.is_some() {
            self.next += 1;
        }
        Ok(block)
    }

    /// Stop reading, releasing any resources held by this reader.
    pub fn close(self) {
        // dropping the reader is enough
    }
}

impl Stream for Reader {
    type Item = Result<Block, ReadError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let poll = this.stream.as_mut().poll_next(cx);
        match poll {
            Poll::Ready(Some(Ok(block))) => {
                this.next += 1;
                Poll::Ready(Some(Ok(block)))
            }
            other => other,
        }
    }
}
