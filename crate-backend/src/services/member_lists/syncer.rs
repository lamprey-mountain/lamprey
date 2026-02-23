use std::collections::VecDeque;
use std::sync::Arc;

use common::v1::types::{MessageSync, UserId};
use tokio::sync::oneshot;
use tokio_stream::{wrappers::BroadcastStream, StreamExt, StreamMap, StreamNotifyClose};
use uuid::Uuid;

use crate::{
    services::member_lists::{
        actor::{MemberListCommand, MemberListEvent},
        util::{MemberListKey, MemberListKey1},
    },
    Error, Result, ServerStateInner,
};

/// Syncer for member list events
pub struct MemberListSyncer {
    pub(super) s: Arc<ServerStateInner>,
    pub(super) conn_id: Uuid,
    pub(super) outbox: VecDeque<MessageSync>,
    pub(super) streams:
        StreamMap<MemberListKey, StreamNotifyClose<BroadcastStream<MemberListEvent>>>,
}

impl MemberListSyncer {
    /// Create a new member list syncer
    pub(super) fn new(s: Arc<ServerStateInner>, conn_id: Uuid) -> Self {
        Self {
            s,
            conn_id,
            outbox: VecDeque::new(),
            streams: StreamMap::new(),
        }
    }

    /// Subscribe to a member list
    pub async fn subscribe(&mut self, key1: MemberListKey1, ranges: Vec<(u64, u64)>) -> Result<()> {
        let srv = self.s.services();
        let key = srv.member_lists.lookup_member_key(key1).await?;

        if self.streams.contains_key(&key) {
            return Ok(());
        }

        let list = srv.member_lists.ensure(key.clone()).await?;

        let (tx, rx) = oneshot::channel();
        list.commands_tx
            .send(MemberListCommand::GetInitialRanges {
                ranges,
                conn_id: self.conn_id,
                callback: tx,
            })
            .await
            .map_err(|_| {
                Error::Internal("failed to send command to member list actor".to_string())
            })?;

        let initial_sync = rx
            .await
            .map_err(|_| Error::Internal("failed to receive initial ranges".to_string()))?;
        self.outbox.push_back(initial_sync);

        let stream = StreamNotifyClose::new(BroadcastStream::new(list.subscribe()));
        self.streams.insert(key, stream);

        Ok(())
    }

    /// Unsubscribe from a member list
    pub async fn unsubscribe(&mut self, key1: MemberListKey1) -> Result<()> {
        let srv = self.s.services();
        let key = srv.member_lists.lookup_member_key(key1).await?;
        self.streams.remove(&key);
        Ok(())
    }

    /// Poll for new events
    pub async fn poll(&mut self, user_id: UserId) -> Result<Option<MessageSync>> {
        loop {
            if let Some(mut msg) = self.outbox.pop_front() {
                self.patch_msg(&mut msg, user_id);
                return Ok(Some(msg));
            }

            tokio::select! {
                Some((_key, val)) = self.streams.next() => {
                    match val {
                        Some(Ok(MemberListEvent::Broadcast(mut msg))) => {
                            self.patch_msg(&mut msg, user_id);
                            return Ok(Some(msg));
                        }
                        Some(Ok(MemberListEvent::Unicast(conn_id, mut msg))) if conn_id == self.conn_id => {
                            self.patch_msg(&mut msg, user_id);
                            return Ok(Some(msg));
                        }
                        Some(Ok(_)) => continue, // skip other unicasts
                        Some(Err(e)) => return Err(Error::Internal(format!("member list stream error: {e}"))),
                        None => continue, // stream closed, try next
                    }
                }
                else => return Ok(None),
            }
        }
    }

    fn patch_msg(&self, msg: &mut MessageSync, user_id: UserId) {
        if let MessageSync::MemberListSync {
            user_id: ref mut uid,
            ..
        } = msg
        {
            *uid = user_id;
        }
    }
}
