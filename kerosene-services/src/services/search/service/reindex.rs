use crate::Result;
use crate::services::search::schema::Doctype;
use crate::services::search::{ServiceSearch, util::SCHEMA};
use lamprey_backend_core::types::data::SearchReindexQueueTarget;
use lamprey_backend_core::types::search::Reindex;
use tantivy::Term;

impl ServiceSearch {
    /// reindex some content
    pub async fn reindex(&self, reindex: Reindex) -> Result<()> {
        let index = self.get_index().await?;
        let mut data = self.state.data();

        // 1. Delete terms based on filters
        for doctype in &reindex.doctypes {
            index
                .delete_term(Term::from_field_text(SCHEMA.doctype, doctype.as_str()))
                .await?;
        }
        for room_id in &reindex.room_ids {
            index
                .delete_term(Term::from_field_text(SCHEMA.room_id, &room_id.to_string()))
                .await?;
        }
        for channel_id in &reindex.channel_ids {
            index
                .delete_term(Term::from_field_text(
                    SCHEMA.channel_id,
                    &channel_id.to_string(),
                ))
                .await?;
        }

        // 2. Update db
        if reindex.doctypes.contains(&Doctype::Message) {
            if reindex.channel_ids.is_empty() {
                if reindex.room_ids.is_empty() {
                    data.search_reindex_queue_reset_all_messages().await?;
                } else {
                    for room_id in &reindex.room_ids {
                        // TODO: only reindex messages
                        data.search_reindex_queue_reset_room(*room_id).await?;
                    }
                }
            } else {
                for channel_id in &reindex.channel_ids {
                    data.search_reindex_queue_upsert(
                        SearchReindexQueueTarget::Messages(*channel_id),
                        None,
                    )
                    .await?;
                }

                // TODO: handle reindex.room_ids
            }
        }

        if reindex.doctypes.contains(&Doctype::AuditLogEntry) {
            if reindex.room_ids.is_empty() {
                data.search_reindex_queue_reset_all_audit_logs().await?;
            } else {
                for room_id in &reindex.room_ids {
                    data.search_reindex_queue_upsert(
                        SearchReindexQueueTarget::AuditLogEntries(*room_id),
                        None,
                    )
                    .await?;
                }
            }
        }

        for doctype in &reindex.doctypes {
            // TODO: handle reindex.channel_ids, reindex.room_ids for these
            let target = match doctype {
                Doctype::Room => Some(SearchReindexQueueTarget::Rooms),
                Doctype::Channel => Some(SearchReindexQueueTarget::Channels),
                Doctype::User => Some(SearchReindexQueueTarget::Users),
                Doctype::Media => Some(SearchReindexQueueTarget::Media),
                _ => None,
            };
            if let Some(target) = target {
                data.search_reindex_queue_upsert(target, None).await?;
            }
        }

        data.commit().await?;

        Ok(())
    }
}
