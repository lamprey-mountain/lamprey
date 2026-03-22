use common::v1::types::search::{
    ChannelSearchOrderField, ChannelSearchRequest, MessageSearchOrderField, MessageSearchRequest,
    Order,
};
use common::v1::types::util::Time;
use common::v1::types::{ChannelId, MessageId, RoomId};
use lamprey_backend_core::prelude::*;
use tantivy::query::{QueryParser, TermSetQuery};
use tantivy::{
    collector::{Count, TopDocs},
    query::{BooleanQuery, Query},
    schema::Value,
    DocAddress, IndexReader, TantivyDocument, Term,
};
use tracing::warn;

use crate::services::search::schema::content::ContentSchema;

pub struct ContentSearcher {
    reader: IndexReader,
    schema: ContentSchema,
}

impl ContentSearcher {
    pub fn new(reader: IndexReader, schema: ContentSchema) -> Self {
        Self { reader, schema }
    }
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

impl ContentSearcher {
    /// generate a tantivy query to restrict visibility
    pub fn generate_visibility_query(
        &self,
        visible_channel_ids: &[(ChannelId, bool)],
    ) -> BooleanQuery {
        let mut channel_terms = vec![];
        let mut parent_channel_terms = vec![];
        for (id, can_view_private_threads) in visible_channel_ids {
            let id_str = id.to_string();
            channel_terms.push(Term::from_field_text(self.schema.channel_id, &id_str));

            if *can_view_private_threads {
                parent_channel_terms.push(Term::from_field_text(
                    self.schema.parent_channel_id,
                    &id_str,
                ));
            }
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

        BooleanQuery::new(vis_queries)
    }

    pub fn search_messages(&self, msg: SearchMessages) -> Result<SearchMessagesResponseRaw> {
        // very unlikely, but might as well
        if msg.visible_channel_ids.is_empty() {
            return Ok(SearchMessagesResponseRaw {
                items: vec![],
                total: 0,
            });
        }

        let searcher = self.reader.searcher();
        let mut query_clauses: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];

        if let Some(q_str) = &msg.req.query {
            if !q_str.is_empty() {
                let mut query_parser = QueryParser::for_index(
                    searcher.index(),
                    vec![self.schema.content, self.schema.name],
                );

                // i know this is useless for messages, this is for reference
                query_parser.set_field_boost(self.schema.name, 2.0);

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                query_clauses.push((tantivy::query::Occur::Must, parsed_query));
            }
        }

        query_clauses.push((
            tantivy::query::Occur::Must,
            Box::new(self.generate_visibility_query(&msg.visible_channel_ids)),
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

            let Some(id) = retrieved_doc
                .get_first(self.schema.id)
                .and_then(|v| v.as_str())
            else {
                warn!("Document missing id field: {:?}", doc_address);
                continue;
            };

            let Some(channel_id) = retrieved_doc
                .get_first(self.schema.channel_id)
                .and_then(|v| v.as_str())
            else {
                warn!("Document missing channel id field: {:?}", doc_address);
                continue;
            };

            items.push(SearchMessagesResponseRawItem {
                id: id.parse().unwrap(),
                channel_id: channel_id.parse().unwrap(),
            });
        }

        Ok(SearchMessagesResponseRaw { items, total })
    }

    // pub fn search_channels(&self, q: SearchChannels) -> Result<_> {}

    pub fn count_documents_for_channel(&self, channel_id: ChannelId) -> Result<u64> {
        let searcher = self.reader.searcher();

        let query = tantivy::query::TermQuery::new(
            Term::from_field_text(self.schema.channel_id, &channel_id.to_string()),
            tantivy::schema::IndexRecordOption::Basic,
        );

        let count = searcher.search(&query, &Count)?;
        Ok(count as u64)
    }
}

pub struct SearchChannels {
    pub req: ChannelSearchRequest,
    pub visible_room_ids: Vec<RoomId>,
}

pub struct SearchChannelsResponseRawItem {
    pub id: ChannelId,
    pub archived_at: Option<Time>,
    pub created_at: Time,
}

pub struct SearchChannelsResponseRaw {
    pub items: Vec<SearchChannelsResponseRawItem>,
    pub total: u64,
}

impl ContentSearcher {
    /// generate a tantivy query to restrict channel visibility by room
    pub fn generate_channel_visibility_query(&self, visible_room_ids: &[RoomId]) -> BooleanQuery {
        let room_terms: Vec<Term> = visible_room_ids
            .iter()
            .map(|id| Term::from_field_text(self.schema.room_id, &id.to_string()))
            .collect();

        let mut vis_queries: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];

        if !room_terms.is_empty() {
            vis_queries.push((
                tantivy::query::Occur::Should,
                Box::new(TermSetQuery::new(room_terms)),
            ));
        }

        BooleanQuery::new(vis_queries)
    }

    pub fn search_channels(&self, msg: SearchChannels) -> Result<SearchChannelsResponseRaw> {
        if msg.visible_room_ids.is_empty() {
            return Ok(SearchChannelsResponseRaw {
                items: vec![],
                total: 0,
            });
        }

        let searcher = self.reader.searcher();
        let mut query_clauses: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];

        // Text query on name and content (description)
        if let Some(q_str) = &msg.req.query {
            if !q_str.is_empty() {
                let mut query_parser = QueryParser::for_index(
                    searcher.index(),
                    vec![self.schema.content, self.schema.name],
                );
                query_parser.set_field_boost(self.schema.name, 2.0);

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                query_clauses.push((tantivy::query::Occur::Must, parsed_query));
            }
        }

        // Filter by doctype = Channel
        let doctype_term = Term::from_field_text(self.schema.doctype, "Channel");
        query_clauses.push((
            tantivy::query::Occur::Must,
            Box::new(tantivy::query::TermQuery::new(
                doctype_term,
                tantivy::schema::IndexRecordOption::Basic,
            )),
        ));

        // Visibility filter
        query_clauses.push((
            tantivy::query::Occur::Must,
            Box::new(self.generate_channel_visibility_query(&msg.visible_room_ids)),
        ));

        let query = BooleanQuery::new(query_clauses);

        let limit = msg.req.limit as usize;
        let cursor = msg.req.offset as usize;

        let (top_docs, total) = match (msg.req.sort_field, msg.req.sort_order) {
            (ChannelSearchOrderField::Relevancy, _) => {
                let (top_docs, count): (Vec<(f32, DocAddress)>, usize) = searcher
                    .search(
                        &query,
                        &(TopDocs::with_limit(limit).and_offset(cursor), Count),
                    )
                    .expect("search failed");
                let top_docs: Vec<DocAddress> = top_docs.into_iter().map(|(_, doc)| doc).collect();
                (top_docs, count as u64)
            }
            (ChannelSearchOrderField::Created, ord) => {
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
            (ChannelSearchOrderField::Archived, ord) => {
                let (top_docs, count): (Vec<(tantivy::DateTime, DocAddress)>, usize) = searcher
                    .search(
                        &query,
                        &(
                            TopDocs::with_limit(limit)
                                .and_offset(cursor)
                                .order_by_fast_field::<tantivy::DateTime>(
                                    "archived_at",
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
            (ChannelSearchOrderField::Activity, ord) => {
                // the caller needs to reorder based on actual activity
                let (top_docs, count): (Vec<(tantivy::DateTime, DocAddress)>, usize) = searcher
                    .search(
                        &query,
                        &(
                            TopDocs::with_limit(limit)
                                .and_offset(cursor)
                                .order_by_fast_field::<tantivy::DateTime>(
                                    "archived_at",
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

            let Some(id) = retrieved_doc
                .get_first(self.schema.id)
                .and_then(|v| v.as_str())
            else {
                warn!("Document missing id field: {:?}", doc_address);
                continue;
            };

            let created_at = retrieved_doc
                .get_first(self.schema.created_at)
                .and_then(|v| v.as_datetime())
                .map(|d| Time::from(d.into_utc()))
                .expect("Document missing created_at");

            let archived_at = retrieved_doc
                .get_first(self.schema.archived_at)
                .and_then(|v| v.as_datetime())
                .map(|d| Time::from(d.into_utc()));

            items.push(SearchChannelsResponseRawItem {
                id: id.parse().unwrap(),
                created_at,
                archived_at,
            });
        }

        Ok(SearchChannelsResponseRaw { items, total })
    }
}
