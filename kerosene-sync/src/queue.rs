use std::collections::VecDeque;

use common::v1::types::{ConnectionId, MessageEnvelope, MessagePayload, MessageSync};
use tracing::{debug, trace};

use crate::error::{Error, Result};
use crate::sync::transport::TransportSink;

#[derive(Debug)]
pub struct ConnectionQueue {
    /// the outbound queue's events
    queue: VecDeque<(Option<u64>, MessageEnvelope)>,

    /// the seq of the last event the server queued
    seq_server: u64,

    /// the seq of the last event the client received
    seq_client: u64,

    /// the maximum number of events to keep in this queue
    max_len: usize,
}

impl ConnectionQueue {
    /// create a new queue
    pub fn new(max_len: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            seq_server: 0,
            seq_client: 0,
            max_len,
        }
    }

    /// attempt to rewind this queue to a seq number
    pub fn rewind(&mut self, seq: u64) -> Result<()> {
        let min_seq = self.seq_server.saturating_sub(self.max_len as u64);
        if seq >= min_seq && seq <= self.seq_server {
            self.seq_client = seq;
            Ok(())
        } else {
            Err(Error::BadStatic("invalid seq"))
        }
    }

    /// checks if the client can resume, assuming they have the latest seq
    pub fn can_resume(&self) -> bool {
        self.rewind_valid(self.seq_client)
    }

    /// checks if events would be missed if the client starts tailing from this sequence number
    fn rewind_valid(&self, seq: u64) -> bool {
        let min_seq = self.seq_server.saturating_sub(self.max_len as u64);
        seq >= min_seq && seq <= self.seq_server
    }

    /// push a new message to this queue
    pub fn push(&mut self, mut msg: MessageEnvelope) {
        trace!(
            "push message seq_server={} name={}",
            self.seq_server,
            msg.payload.name()
        );

        let ephemeral = match &mut msg.payload {
            MessagePayload::Ping => true,
            MessagePayload::Sync { .. } => false,
            MessagePayload::Error { .. } => true,
            MessagePayload::Ready { .. } => false,
            MessagePayload::Resumed => true,
            MessagePayload::Reconnect { .. } => true,
        };

        // we need to increment self_seq BEFORE sending the message to handle
        // when seq_server is 0. the event would never be sent to the client
        // because the last seen event will always be at least 0.
        //
        // this means that seq numbers always start counting from 1, and 0
        // is used to mean the server hasn't sent anything, the client hasn't
        // received anything yet
        if !ephemeral {
            self.seq_server += 1;
        }

        match &mut msg.payload {
            MessagePayload::Sync { seq, .. } => {
                *seq = self.seq_server;
            }
            MessagePayload::Ready { seq, .. } => {
                *seq = self.seq_server;
            }
            _ => {}
        };

        let seq_tag = if ephemeral {
            None
        } else {
            Some(self.seq_server)
        };

        self.queue.push_front((seq_tag, msg));
        self.queue.truncate(self.max_len);
    }

    /// push a new message to this queue without a seq number
    pub fn push_ephemeral(&mut self, msg: MessageEnvelope) {
        debug!("push ephemeral message {:?}", msg);
        self.queue.push_front((None, msg));
        self.queue.truncate(self.max_len);
    }

    /// shortcut for push a new message to this queue
    pub fn push_sync(&mut self, sync: MessageSync, nonce: Option<String>) {
        self.push(MessageEnvelope {
            payload: MessagePayload::Sync {
                data: Box::new(sync),
                seq: 0, // will be overwritten by push()
                nonce,
            },
        })
    }

    /// drain this queue into this transport
    #[tracing::instrument(level = "debug", skip(self, transport), fields(id = connection_id.to_string()))]
    pub async fn drain(
        &mut self,
        transport: &mut dyn TransportSink,
        connection_id: ConnectionId,
    ) -> Result<()> {
        trace!(?self.seq_client, ?self.seq_server, queue_len = self.queue.len(), "before");

        let last_seen = self.seq_client;
        let queue = &self.queue;

        for (seq, msg) in queue.iter().rev() {
            trace!(?seq, "process");

            if seq.is_some_and(|s| s <= last_seen) {
                continue;
            }

            transport.send(msg.clone()).await?;

            if let Some(s) = *seq {
                self.seq_client = self.seq_client.max(s);
            }
        }

        // clear ephemeral messages
        self.queue.retain(|(seq, _)| seq.is_some());

        trace!(?self.seq_client, ?self.seq_server, queue_len = self.queue.len(), "after");

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
