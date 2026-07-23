use crate::{Error, Result};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::{RepliesChildren, RepliesMessage};
use common::v2::types::media::MediaReference;
use common::v2::types::{MediaId, MessageId};
use lamprey_backend_data_postgres::MessageWithCounts;
use std::collections::HashSet;

/// utility for enforcing deduplication of media
#[derive(Default)]
pub struct MediaRegistry {
    pub known: HashSet<MediaId>,
    pub duplicates: HashSet<MediaId>,
}

impl MediaRegistry {
    pub fn insert(&mut self, media_id: MediaId) {
        if !self.known.insert(media_id) {
            self.duplicates.insert(media_id);
        }
    }

    pub fn insert_ref(&mut self, mr: &MediaReference) -> Result<()> {
        let Some(media_id) = mr.media_id() else {
            return Err(Error::Unimplemented);
        };

        self.insert(media_id);
        Ok(())
    }

    // PERF: this could probably be improved
    pub fn extend(&mut self, ids: &[MediaId]) {
        for id in ids {
            self.insert(*id);
        }
    }

    pub fn extend_refs(&mut self, items: &[MediaReference]) {
        for i in items {
            let _ = self.insert_ref(i);
        }
    }

    pub fn check(&self) -> Result<()> {
        if self.duplicates.is_empty() {
            Ok(())
        } else {
            let dupes: Vec<_> = self.duplicates.iter().map(|m| m.to_string()).collect();
            Err(Error::ApiError(ApiError::with_message(
                ErrorCode::DuplicateMediaId,
                format!(
                    "You've used some media ids multiple times, but media can only be used once. Media ids: {}",
                    dupes.join(", ")
                ),
            )))
        }
    }
}

/// utility to build a tree from a list of messages
pub struct TreeBuilder<'a> {
    messages: &'a [MessageWithCounts],
    max_depth: u16,
}

impl<'a> TreeBuilder<'a> {
    pub fn new(messages: &'a [MessageWithCounts], max_depth: u16) -> Self {
        Self {
            messages,
            max_depth,
        }
    }

    pub fn build(&self, parent_id: Option<MessageId>, depth: u16) -> RepliesChildren {
        let children: Vec<_> = self
            .messages
            .iter()
            .filter(|msg| msg.message.reply_id() == parent_id)
            .map(|msg| {
                let (count_direct, count_recursive) = (msg.count_direct, msg.count_recursive);

                let subtree = if depth < self.max_depth {
                    self.build(Some(msg.message.id), depth + 1)
                } else {
                    RepliesChildren {
                        children: vec![],
                        count_direct,
                        count_recursive,
                        depth: (depth + 1) as u64,
                        cursor: None,
                        has_more: false,
                    }
                };

                RepliesMessage {
                    message: msg.message.clone(),
                    children: RepliesChildren {
                        count_direct,
                        count_recursive,
                        ..subtree
                    },
                }
            })
            .collect();

        RepliesChildren {
            count_direct: children.len() as u64,
            count_recursive: 0,
            children,
            depth: depth as u64,
            cursor: None,
            has_more: false,
        }
    }
}
