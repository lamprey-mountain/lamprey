use common::v1::types::{MemberListGroup, MessageSync, UserId};

use crate::cache::Cache;

pub struct MemberList {
    cache: Cache,

    groups: Vec<MemberListGroup>,

    /// ranges of members in this list
    items: Vec<Vec<UserId>>,
}

impl MemberList {
    pub fn handle_sync(&mut self, msg: MessageSync) {
        let MessageSync::MemberListSync { .. } = msg else {
            return;
        };

        todo!()
    }
}
