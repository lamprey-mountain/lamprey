use common::v1::types::search::{MessageSearchOrderField, MessageSearchRequest, Order};
use common::v1::types::{ChannelId, MessageId};
use lamprey_backend_core::prelude::*;
use tantivy::query::TermSetQuery;
use tantivy::{
    collector::{Count, TopDocs},
    query::{BooleanQuery, Query},
    schema::Value,
    DocAddress, IndexReader, TantivyDocument, Term,
};

use crate::services::search::schema::content::ContentSchema;

use super::index::IndexActorRef;

// TEMP: public
pub struct MessageSearcher {
    pub(crate) index_ref: IndexActorRef,
    pub(crate) reader: IndexReader,
    pub(crate) schema: ContentSchema,
}

pub struct SearchMessages {
    pub req: MessageSearchRequest,
    pub visible_channel_ids: Vec<(ChannelId, bool)>,
}

pub struct SearchMessagesResponseRawItem {
    pub id: MessageId,
    pub channel_id: ChannelId,
}

pub struct SearchMessagesResponseRaw {
    pub items: Vec<SearchMessagesResponseRawItem>,
    pub total: u64,
}

impl MessageSearcher {
    pub fn search_messages(&self, msg: SearchMessages) -> Result<SearchMessagesResponseRaw> {
        let searcher = self.reader.searcher();
        let mut query_clauses: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];

        // Build query from request
        if let Some(q_str) = &msg.req.query {
            if !q_str.is_empty() {
                // TODO: need index reference for QueryParser
                // For now, skip full-text query building
            }
        }

        // restrict visibility
        let mut channel_terms = vec![];
        let mut parent_channel_terms = vec![];
        for (id, can_view_private_threads) in &msg.visible_channel_ids {
            let id_str = id.to_string();
            channel_terms.push(Term::from_field_text(self.schema.channel_id, &id_str));

            if *can_view_private_threads {
                parent_channel_terms.push(Term::from_field_text(
                    self.schema.parent_channel_id,
                    &id_str,
                ));
            }
        }

        if channel_terms.is_empty() && parent_channel_terms.is_empty() {
            return Ok(SearchMessagesResponseRaw {
                items: vec![],
                total: 0,
            });
        }

        let mut vis_queries: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];

        if !channel_terms.is_empty() {
            vis_queries.push((
                tantivy::query::Occur::Should,
                Box::new(TermSetQuery::new(channel_terms)),
            ));
        }

        if !parent_channel_terms.is_empty() {
            vis_queries.push((
                tantivy::query::Occur::Should,
                Box::new(TermSetQuery::new(parent_channel_terms)),
            ));
        }

        query_clauses.push((
            tantivy::query::Occur::Must,
            Box::new(BooleanQuery::new(vis_queries)),
        ));

        let query = BooleanQuery::new(query_clauses);

        let limit = msg.req.limit as usize;
        let cursor = msg.req.offset as usize;

        let (top_docs, total) = match (msg.req.sort_field, msg.req.sort_order) {
            (MessageSearchOrderField::Relevancy, _) => {
                let (top_docs, count): (Vec<(f32, DocAddress)>, usize) = searcher
                    .search(
                        &query,
                        &(TopDocs::with_limit(limit).and_offset(cursor), Count),
                    )
                    .expect("search failed");
                let top_docs: Vec<DocAddress> = top_docs.into_iter().map(|(_, doc)| doc).collect();
                (top_docs, count as u64)
            }
            (MessageSearchOrderField::Created, ord) => {
                let (top_docs, count): (Vec<(tantivy::DateTime, DocAddress)>, usize) = searcher
                    .search(
                        &query,
                        &(
                            TopDocs::with_limit(limit)
                                .and_offset(cursor)
                                .order_by_fast_field::<tantivy::DateTime>(
                                    "created_at",
                                    match ord {
                                        Order::Ascending => tantivy::Order::Asc,
                                        Order::Descending => tantivy::Order::Desc,
                                    },
                                ),
                            Count,
                        ),
                    )
                    .expect("search failed");
                let top_docs: Vec<DocAddress> = top_docs.into_iter().map(|(_, doc)| doc).collect();
                (top_docs, count as u64)
            }
        };

        let mut items = vec![];
        for doc_address in top_docs {
            let retrieved_doc: TantivyDocument =
                searcher.doc(doc_address).expect("doc fetch failed");
            let id = retrieved_doc
                .get_first(self.schema.id)
                .unwrap()
                .as_str()
                .unwrap();
            let channel_id = retrieved_doc
                .get_first(self.schema.channel_id)
                .unwrap()
                .as_str()
                .unwrap();
            items.push(SearchMessagesResponseRawItem {
                id: id.parse().unwrap(),
                channel_id: channel_id.parse().unwrap(),
            });
        }

        Ok(SearchMessagesResponseRaw { items, total })
    }
}
