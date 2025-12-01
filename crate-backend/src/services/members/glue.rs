use std::sync::Arc;

use common::v1::types::{MemberListGroup, MemberListOp, MessageSync, PermissionOverwrites};
use dashmap::DashMap;
use tokio::sync::broadcast;
use tracing::warn;

use crate::{
    services::members::{
        lists::MemberList2,
        util::{MemberListKey, MemberListVisibility},
        MemberList, MemberListTarget, ServiceMembers,
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
    pub fn create_syncer(&self) -> MemberListSyncer {
        todo!()
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

            loop {
                let msg = events.recv().await.expect("error while receiving event");
                let ops = match msg {
                    MessageSync::ChannelUpdate { channel } => {
                        // TODO: optimize
                        let srv = s.services();
                        let mut overwrites = vec![channel.permission_overwrites.clone()];
                        let mut top = channel;
                        while let Some(parent_id) = top.parent_id {
                            // TODO: handle error
                            let chan = srv
                                .channels
                                .get(parent_id, None)
                                .await
                                .expect("failed to fetch channel");
                            overwrites.push(chan.permission_overwrites.clone());
                            top = Box::new(chan);
                        }
                        overwrites.reverse();
                        let v = MemberListVisibility { overwrites };
                        list.set_visibility(v)
                    }
                    msg => list.process(&msg),
                };
                if !ops.is_empty() {
                    // TODO: handle error
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
    pub fn set_query(&mut self, target: MemberListTarget, ranges: &[(u64, u64)]) {
        todo!()
    }

    /// poll for the next member list sync message
    pub async fn poll(&self) -> Result<MessageSync> {
        todo!()
    }
}
