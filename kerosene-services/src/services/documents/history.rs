use std::collections::HashSet;

use common::v1::types::document::{Changeset, DocumentTag, HistoryParams};
use common::v2::types::{ChannelId, UserId};
use kerosene_core::types::documents::EditContextId;
use lamprey_backend_data_postgres::DocumentUpdateSummary;

use crate::prelude::*;
use crate::services::documents::ServiceDocuments;
use crate::services::documents::util::HistoryPaginationSummary;

impl ServiceDocuments {
    pub async fn query_history(
        &self,
        context_id: EditContextId,
        query: HistoryParams,
    ) -> Result<HistoryPaginationSummary> {
        let (updates, tags) = self
            .globals
            .begin_read()
            .await?
            .document_history(context_id)
            .await?;
        self.process_history(updates, tags, query)
    }

    pub async fn query_wiki_history(
        &self,
        wiki_id: ChannelId,
        query: HistoryParams,
    ) -> Result<HistoryPaginationSummary> {
        let (updates, tags) = self
            .globals
            .begin_read()
            .await?
            .wiki_history(wiki_id)
            .await?;
        self.process_history(updates, tags, query)
    }

    fn process_history(
        &self,
        updates: Vec<DocumentUpdateSummary>,
        tags: Vec<DocumentTag>,
        query: HistoryParams,
    ) -> Result<HistoryPaginationSummary> {
        // PERF: create some sort of streaming history processor

        let by_author = query.by_author.unwrap_or(true);
        let by_tag = query.by_tag.unwrap_or(true);
        let by_time = query.by_time.unwrap_or(3600) as i64;
        let by_changes = query.by_changes.unwrap_or(100) as usize;

        let mut changesets = Vec::new();
        if updates.is_empty() {
            return Ok(HistoryPaginationSummary {
                changesets,
                tags: vec![],
            });
        }

        // FIXME: added/removed should be calculated from the actual diff rather than by summing each individual changeset

        let mut current_authors = HashSet::new();
        let mut current_added = 0;
        let mut current_removed = 0;
        let mut current_start = updates[0].created_at;
        let mut current_end = updates[0].created_at;
        let mut current_count = 0;
        let mut current_document_id = updates[0].document_id;
        let mut current_start_seq = updates[0].seq;
        let mut current_end_seq = updates[0].seq;

        let mut tag_iter = tags.iter().peekable();

        for (i, update) in updates.iter().enumerate() {
            let mut split = false;

            if i > 0 {
                let prev = &updates[i - 1];

                if update.document_id != prev.document_id {
                    split = true;
                }

                if by_author && update.user_id != prev.user_id {
                    split = true;
                }

                let diff = (*update.created_at - *prev.created_at).whole_seconds();
                if diff > by_time {
                    split = true;
                }

                if current_count >= by_changes {
                    split = true;
                }

                if by_tag {
                    while let Some(tag) = tag_iter.peek() {
                        if tag.revision_seq < prev.seq as u64 {
                            tag_iter.next();
                            continue;
                        }
                        if tag.revision_seq == prev.seq as u64 {
                            split = true;
                        }
                        break;
                    }
                }
            }

            if split {
                changesets.push(Changeset {
                    start_time: current_start,
                    end_time: current_end,
                    authors: current_authors.drain().collect(),
                    stat_added: current_added,
                    stat_removed: current_removed,
                    document_id: Some(current_document_id),
                    start_seq: current_start_seq,
                    end_seq: current_end_seq,
                });
                current_added = 0;
                current_removed = 0;
                current_count = 0;
                current_start = update.created_at;
                current_start_seq = update.seq;
                current_document_id = update.document_id;
            }

            current_authors.insert(UserId::from(update.user_id));
            current_added += update.stat_added as u64;
            current_removed += update.stat_removed as u64;
            current_end = update.created_at;
            current_end_seq = update.seq;
            current_count += 1;
        }

        changesets.push(Changeset {
            start_time: current_start,
            end_time: current_end,
            authors: current_authors.drain().collect(),
            stat_added: current_added,
            stat_removed: current_removed,
            document_id: Some(current_document_id),
            start_seq: current_start_seq,
            end_seq: current_end_seq,
        });

        changesets.reverse();

        if let Some(limit) = query.limit {
            changesets.truncate(limit as usize);
        } else {
            changesets.truncate(20);
        }

        Ok(HistoryPaginationSummary { changesets, tags })
    }
}
