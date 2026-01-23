use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

use common::v1::types::{ChannelId, MemberListOp, MessageSync, RoleId, RoomId, UserId};
use dashmap::DashMap;
use tokio::{
    sync::{mpsc::Receiver, oneshot},
    task::JoinHandle,
};
use tokio_stream::{wrappers::BroadcastStream, StreamExt, StreamMap, StreamNotifyClose};
use uuid::Uuid;

use crate::{
    services::member_lists::{
        actor::{MemberListCommand, MemberListEvent},
        util::{MemberKey, MemberListGroupData, MemberListKey, MemberListKey1},
    },
    Result, ServerStateInner,
};

pub struct MemberListSyncer {
    pub(super) s: Arc<ServerStateInner>,
    pub(super) conn_id: Uuid,
    pub(super) outbox: VecDeque<MessageSync>,
    // pub(super) key_map: HashMap<MemberListKey1, MemberListKey>,
    pub(super) streams:
        StreamMap<MemberListKey, StreamNotifyClose<BroadcastStream<MemberListEvent>>>,
}

// NOTE: user_id is removed, auth checks should be done in syncer
// NOTE: how do i handle initial ranges with updates?
// - prevent receiving updates until after i have initial ranges
// - prevent skipped/missed updates after i have initial ranges
// TODO: replace with actual member list service
impl MemberListSyncer {
    pub(super) fn new(s: Arc<ServerStateInner>, conn_id: Uuid) -> Self {
        Self {
            s,
            conn_id,
            outbox: VecDeque::new(),
            streams: StreamMap::new(),
        }
    }

    /// subscribe to a new member list
    pub async fn subscribe(&self, key1: MemberListKey1, ranges: Vec<(u64, u64)>) -> Result<()> {
        let srv = self.s.services();
        // srv.member_list
        let member_list: super::ServiceMemberLists = todo!();
        let key = member_list.lookup_member_key(key1).await?;

        // FIXME: don't subscribe if we're already subscribed via another key1
        let list = member_list.ensure(key.clone()).await?;

        let (tx, rx) = oneshot::channel();
        list.commands
            .send(MemberListCommand::GetInitialRanges {
                ranges,
                conn_id: self.conn_id,
                callback: tx,
            })
            .await;
        // TODO: better error handling instead of unwrap
        self.outbox.push_back(rx.await.unwrap());

        let stream = StreamNotifyClose::new(BroadcastStream::new(list.subscribe()));
        self.streams.insert(key, stream);

        Ok(())
    }

    /// unsubscribe from a member list
    pub async fn unsubscribe(&self, key1: MemberListKey1) -> Result<()> {
        // srv.member_list
        let member_list: super::ServiceMemberLists = todo!();
        let key = member_list.lookup_member_key(key1).await?;
        // FIXME: don't unsubscribe from this list if we're subscribed via another key1
        self.streams.remove(&key);
        Ok(())
    }

    /// poll for new events
    pub async fn poll(&mut self) -> Result<MessageSync> {
        if let Some(msg) = self.outbox.pop_front() {
            return Ok(msg);
        }

        while let Some((key, val)) = self.streams.next().await {
            match val {
                Some(val) => println!("got {val:?} from stream {key:?}"),
                None => println!("stream {key:?} closed"),
            }
        }

        todo!()
    }
}
