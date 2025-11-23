use std::sync::Arc;

use common::v1::types::MessageSync;
use dashmap::DashMap;

use crate::{
    services::members::{util::MemberListKey, MemberList, MemberListTarget, ServiceMembers},
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
    list: MemberList,
}

impl ServiceMembers2 {
    pub fn create_syncer(&self) -> MemberListSyncer {
        todo!()
    }

    pub fn create_handler(&self) -> MemberListHandler {
        todo!()
    }
}

impl MemberListHandler {}

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
