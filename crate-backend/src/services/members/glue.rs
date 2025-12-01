use std::sync::Arc;

use common::v1::types::{MemberListGroup, MemberListOp, MessageSync};
use dashmap::DashMap;
use tokio::sync::{broadcast, Mutex};
use tracing::warn;

use crate::{
    services::members::{
        lists::MemberList2,
        util::{MemberListKey, MemberListVisibility},
        MemberListTarget,
    },
    Result, ServerState,
};

pub struct ServiceMembers2 {
    s: Arc<ServerState>,
    lists: DashMap<MemberListKey, Arc<MemberListHandler>>,
}

/// one syncer exists for each connected session
pub struct MemberListSyncer {
    s: Arc<ServerState>,
    query_tx: tokio::sync::watch::Sender<MemberListQuery>,
    query_rx: tokio::sync::watch::Receiver<MemberListQuery>,
    ops_rx: Mutex<Option<broadcast::Receiver<MemberListSync>>>,
}

#[derive(Debug)]
pub struct MemberListQuery {
    target: MemberListTarget,
    ranges: Vec<(u64, u64)>,
}

/// one handler exists for each member list
pub struct MemberListHandler {
    s: Arc<ServerState>,
    list: MemberList2,
    tx: broadcast::Receiver<MemberListSync>,
}

/// minimal member list sync payload for broadcasting
#[derive(Debug, Clone)]
pub struct MemberListSync {
    key: MemberListKey,
    ops: Vec<MemberListOp>,
    groups: Vec<MemberListGroup>,
}

impl ServiceMembers2 {
    /// create a new MemberListSyncer for a session
    pub fn create_syncer(&self, q: MemberListQuery) -> MemberListSyncer {
        let (query_tx, query_rx) = tokio::sync::watch::channel(q);
        MemberListSyncer {
            s: self.s.clone(),
            query_tx,
            query_rx,
            ops_rx: Mutex::new(None),
        }
    }

    /// spawn a handler for a key if it doesnt exist
    // TODO: reuse receivers if they already exist for a key
    // TODO: shutdown unused receivers after a period of time
    pub fn ensure_handler(&self, key: MemberListKey) -> broadcast::Receiver<MemberListSync> {
        let (tx, rx) = broadcast::channel(100);
        let mut events = self.s.sushi.subscribe();
        let s = self.s.clone();

        tokio::spawn(async move {
            let mut list = match MemberList2::new_from_server(key.clone(), &s).await {
                Ok(l) => l,
                Err(e) => {
                    warn!("failed to init member list: {e:?}");
                    return;
                }
            };

            // TODO: better error handling
            loop {
                let msg = events.recv().await.expect("error while receiving event");
                let ops = match msg {
                    MessageSync::ChannelUpdate { channel } => {
                        let srv = s.services();
                        let overwrites = srv
                            .channels
                            .fetch_overwrite_ancestors(channel.id)
                            .await
                            .unwrap();
                        let v = MemberListVisibility { overwrites };
                        list.set_visibility(v)
                    }
                    msg => list.process(&msg),
                };
                if !ops.is_empty() {
                    tx.send(MemberListSync {
                        key: key.clone(),
                        ops,
                        groups: list.groups(),
                    })
                    .unwrap();
                }
            }
        });

        rx
    }
}

impl MemberListSyncer {
    /// set the new query
    pub fn set_query(&self, target: MemberListTarget, ranges: &[(u64, u64)]) {
        self.query_tx
            .send(MemberListQuery {
                target,
                ranges: ranges.to_vec(),
            })
            .unwrap();
    }

    /// poll for the next member list sync message
    // TODO: better error handling
    pub async fn poll(&self) -> Result<MessageSync> {
        let mut qrx = self.query_rx.clone();
        if let Some(ops_rx) = &mut *self.ops_rx.lock().await {
            tokio::select! {
                op = ops_rx.recv() => {
                    // let op = op.unwrap()
                    // Ok(MessageSync::Foo { ... })
                    todo!()
                }
                changed = qrx.changed() => {
                    changed.unwrap();
                    // return list.get_initial_ranges
                    todo!()
                }
            }
        } else {
            qrx.changed().await.unwrap();
            // return list.get_initial_ranges
            todo!()
        }
    }
}
