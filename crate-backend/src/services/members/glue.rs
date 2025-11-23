use std::sync::Arc;

use common::v1::types::{MemberListGroup, MemberListOp, MessageSync};
use dashmap::DashMap;
use tokio::sync::broadcast;

use crate::{
    services::members::{
        lists::MemberList2, util::MemberListKey, MemberList, MemberListTarget, ServiceMembers,
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
    tx: broadcast::Sender<MemberListSync>,
}

#[derive(Clone)]
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
    pub fn ensure_handler(&self, key: MemberListKey) -> MemberListHandler {
        let (tx, _) = broadcast::channel(100);
        MemberListHandler {
            s: self.s.clone(),
            list: todo!(),
            tx,
        }
    }
}

impl MemberListHandler {
    pub fn spawn(mut self) {
        tokio::spawn(async move {
            let mut events = self.s.sushi.subscribe();
            loop {
                let msg = events.recv().await.expect("error while receiving event");
                let ops = self.list.process(&msg);
                if !ops.is_empty() {
                    self.tx.send(MemberListSync {
                        key: self.list.key,
                        ops,
                        groups: self.list.groups(),
                    });
                }
            }
        });
    }

    /// poll for the next member list sync message
    pub async fn poll(&self) -> Result<MessageSync> {
        todo!()
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
