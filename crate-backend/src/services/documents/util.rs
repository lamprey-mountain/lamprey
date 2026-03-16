use common::v1::types::document::{Changeset, DocumentTag};
use common::v1::types::UserId;
use tracing::trace;
use yrs::{GetString, Out};

pub const DOCUMENT_ROOT_NAME: &'static str = "doc";

pub fn get_update_len(v: &Out, txn: &yrs::TransactionMut) -> usize {
    match v {
        Out::Any(yrs::Any::String(s)) => {
            let len = s.chars().count();
            trace!(len = len, "get_update_len matched Any::String");
            len
        }
        Out::YText(t) => {
            let len = t.get_string(txn).chars().count();
            trace!(len = len, "get_update_len matched YText");
            len
        }
        Out::YXmlText(t) => {
            let len = t.get_string(txn).chars().count();
            trace!(len = len, "get_update_len matched YXmlText");
            len
        }
        Out::YXmlElement(e) => {
            let len = e.get_string(txn).chars().count();
            trace!(len = len, "get_update_len matched YXmlElement");
            len
        }
        _ => {
            trace!("get_update_len matched other");
            0
        }
    }
}

pub struct HistoryPaginationSummary {
    pub changesets: Vec<Changeset>,
    pub tags: Vec<DocumentTag>,
}

impl HistoryPaginationSummary {
    /// get a list of all referenced users
    pub fn user_ids(&self) -> Vec<UserId> {
        let mut ids = std::collections::HashSet::new();
        for cs in &self.changesets {
            for author in &cs.authors {
                ids.insert(*author);
            }
        }
        ids.into_iter().collect()
    }
}
