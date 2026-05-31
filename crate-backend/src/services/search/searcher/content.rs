use common::v1::types::search::{
    AuditLogSearchRequest, ChannelSearchOrderField, ChannelSearchRequest, MediaSearchRequest,
    MessageSearchOrderField, MessageSearchRequest, Order, UserSearchRequest,
};
use common::v1::types::util::Time;
use common::v1::types::{
    search::RoomSearchRequest, AuditLogEntryId, ChannelId, MediaId, MessageId, RoomId, UserId,
};
use lamprey_backend_core::prelude::*;
use tantivy::query::{QueryParser, TermSetQuery};
use tantivy::{
    collector::{Count, TopDocs},
    query::{BooleanQuery, Query},
    schema::Value,
    DocAddress, IndexReader, TantivyDocument, Term,
};
use tracing::warn;

use crate::services::search::schema::unified::UnifiedSchema;

pub struct ContentSearcher {
    reader: IndexReader,
    schema: UnifiedSchema,
}

impl ContentSearcher {
    pub fn new(reader: IndexReader, schema: UnifiedSchema) -> Self {
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
    // TODO: move to util?
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
        if msg.visible_channel_ids.is_empty() {
            return Ok(SearchMessagesResponseRaw {
                items: vec![],
                total: 0,
            });
        }

        let searcher = self.reader.searcher();
        let mut query_clauses: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];

        if let Some(q_str) = &msg.req.inner.query {
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

        let doctype_term = Term::from_field_text(self.schema.doctype, "Message");
        query_clauses.push((
            tantivy::query::Occur::Must,
            Box::new(tantivy::query::TermQuery::new(
                doctype_term,
                tantivy::schema::IndexRecordOption::Basic,
            )),
        ));

        query_clauses.push((
            tantivy::query::Occur::Must,
            Box::new(self.generate_visibility_query(&msg.visible_channel_ids)),
        ));

        let query = BooleanQuery::new(query_clauses);

        let limit = msg.req.inner.limit as usize;
        let cursor = msg.req.inner.offset as usize;

        let (top_docs, total) = match (msg.req.sort_field, msg.req.inner.sort_order) {
            (MessageSearchOrderField::Relevancy, _) => {
                let (top_docs, count): (Vec<(f32, DocAddress)>, usize) = searcher
                    .search(
                        &query,
                        &(TopDocs::with_limit(limit).and_offset(cursor), Count),
                    )
                    .map_err(|e| Error::Internal(format!("Search failed: {e}")))?;
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
                    .map_err(|e| Error::Internal(format!("Search failed: {e}")))?;
                let top_docs: Vec<DocAddress> = top_docs.into_iter().map(|(_, doc)| doc).collect();
                (top_docs, count as u64)
            }
        };

        let mut items = vec![];
        for doc_address in top_docs {
            let doc: crate::services::search::index::glue::TantivyMessage = searcher.doc(doc_address)?;
            items.push(SearchMessagesResponseRawItem {
                id: doc.id,
                channel_id: doc.channel_id,
            });
        }

        Ok(SearchMessagesResponseRaw { items, total })
    }
}

pub struct SearchRoomsRaw {
    pub items: Vec<RoomId>,
    pub total: u64,
}

impl ContentSearcher {
    pub fn search_rooms(&self, req: RoomSearchRequest) -> Result<SearchRoomsRaw> {
        let searcher = self.reader.searcher();
        let mut query_clauses: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];

        if let Some(q_str) = &req.inner.query {
            if !q_str.is_empty() {
                let query_parser = QueryParser::for_index(searcher.index(), vec![self.schema.name]);

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                query_clauses.push((tantivy::query::Occur::Must, parsed_query));
            }
        }

        // filter by doctype = Room
        let doctype_term = Term::from_field_text(self.schema.doctype, "Room");
        query_clauses.push((
            tantivy::query::Occur::Must,
            Box::new(tantivy::query::TermQuery::new(
                doctype_term,
                tantivy::schema::IndexRecordOption::Basic,
            )),
        ));

        let query = BooleanQuery::new(query_clauses);

        let limit = req.inner.limit as usize;
        let cursor = req.inner.offset as usize;

        // TODO: implement sorting
        // TEMP: use relevancy searching
        let (top_docs, total) = searcher
            .search(
                &query,
                &(TopDocs::with_limit(limit).and_offset(cursor), Count),
            )
            .map_err(|e| Error::Internal(format!("Search failed: {e}")))?;

        let top_docs: Vec<DocAddress> = top_docs.into_iter().map(|(_, doc)| doc).collect();

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

            items.push(id.parse().unwrap());
        }

        Ok(SearchRoomsRaw {
            items,
            total: total as u64,
        })
    }

    pub fn count_documents_for_channel(&self, channel_id: ChannelId) -> Result<u64> {
        let searcher = self.reader.searcher();

        let query = tantivy::query::TermQuery::new(
            Term::from_field_text(self.schema.channel_id, &channel_id.to_string()),
            tantivy::schema::IndexRecordOption::Basic,
        );

        let count = searcher.search(&query, &Count)?;
        Ok(count as u64)
    }

    pub fn get_index_stats(&self) -> Result<(u64, u64)> {
        let searcher = self.reader.searcher();
        let num_docs = searcher.num_docs();

        let index = searcher.index();
        let mut total_size = 0;
        // This is a bit of a heuristic for "index size"
        for segment_meta in index.load_metas()?.segments {
            total_size += segment_meta.num_docs() as u64 * 100; // rough guess per doc if we can't get bytes easily
                                                                // Actually tantivy doesn't easily expose byte size of segments in the searcher API without going to directory
        }

        // Let's try to get actual size if possible, or just return num_docs for now
        Ok((num_docs, total_size))
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
        if let Some(q_str) = &msg.req.inner.query {
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

        let limit = msg.req.inner.limit as usize;
        let cursor = msg.req.inner.offset as usize;

        let (top_docs, total) = match (msg.req.sort_field, msg.req.inner.sort_order) {
            (ChannelSearchOrderField::Relevancy, _) => {
                let (top_docs, count): (Vec<(f32, DocAddress)>, usize) = searcher
                    .search(
                        &query,
                        &(TopDocs::with_limit(limit).and_offset(cursor), Count),
                    )
                    .map_err(|e| Error::Internal(format!("Search failed: {e}")))?;
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
                    .map_err(|e| Error::Internal(format!("Search failed: {e}")))?;
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
                    .map_err(|e| Error::Internal(format!("Search failed: {e}")))?;
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
                    .map_err(|e| Error::Internal(format!("Search failed: {e}")))?;
                let top_docs: Vec<DocAddress> = top_docs.into_iter().map(|(_, doc)| doc).collect();
                (top_docs, count as u64)
            }
            (ChannelSearchOrderField::Name | ChannelSearchOrderField::Id, _) => {
                return Err(Error::Unimplemented)
            } // FIXME: the trait bound `std::string::String: FastValue` is not satisfied: the trait `FastValue` is not implemented for `std::string::String`
              // (ChannelSearchOrderField::Name, ord) => {
              //     let (top_docs, count): (Vec<(String, DocAddress)>, usize) = searcher
              //         .search(
              //             &query,
              //             &(
              //                 TopDocs::with_limit(limit)
              //                     .and_offset(cursor)
              //                     .order_by_fast_field::<String>(
              //                         "name",
              //                         match ord {
              //                             Order::Ascending => tantivy::Order::Asc,
              //                             Order::Descending => tantivy::Order::Desc,
              //                         },
              //                     ),
              //                 Count,
              //             ),
              //         )
              //         .map_err(|e| Error::Internal(format!("Search failed: {e}")))?;
              //     let top_docs: Vec<DocAddress> = top_docs.into_iter().map(|(_, doc)| doc).collect();
              //     (top_docs, count as u64)
              // }
              // (ChannelSearchOrderField::Id, ord) => {
              //     let (top_docs, count): (Vec<(String, DocAddress)>, usize) = searcher
              //         .search(
              //             &query,
              //             &(
              //                 TopDocs::with_limit(limit)
              //                     .and_offset(cursor)
              //                     .order_by_fast_field::<String>(
              //                         "id",
              //                         match ord {
              //                             Order::Ascending => tantivy::Order::Asc,
              //                             Order::Descending => tantivy::Order::Desc,
              //                         },
              //                     ),
              //                 Count,
              //             ),
              //         )
              //         .map_err(|e| Error::Internal(format!("Search failed: {e}")))?;
              //     let top_docs: Vec<DocAddress> = top_docs.into_iter().map(|(_, doc)| doc).collect();
              //     (top_docs, count as u64)
              // }
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

    pub fn search_users(&self, req: UserSearchRequest) -> Result<SearchUsersRaw> {
        let searcher = self.reader.searcher();
        let mut query_clauses: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];

        // Text query on name
        if let Some(q_str) = &req.inner.query {
            if !q_str.is_empty() {
                let query_parser = QueryParser::for_index(searcher.index(), vec![self.schema.name]);

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                query_clauses.push((tantivy::query::Occur::Must, parsed_query));
            }
        }

        // Filter by doctype = User
        let doctype_term = Term::from_field_text(self.schema.doctype, "User");
        query_clauses.push((
            tantivy::query::Occur::Must,
            Box::new(tantivy::query::TermQuery::new(
                doctype_term,
                tantivy::schema::IndexRecordOption::Basic,
            )),
        ));

        let query = BooleanQuery::new(query_clauses);

        let limit = req.inner.limit as usize;
        let cursor = req.inner.offset as usize;

        let (top_docs, total) = searcher
            .search(
                &query,
                &(TopDocs::with_limit(limit).and_offset(cursor), Count),
            )
            .map_err(|e| Error::Internal(format!("Search failed: {e}")))?;

        let top_docs: Vec<DocAddress> = top_docs.into_iter().map(|(_, doc)| doc).collect();

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

            items.push(id.parse().unwrap());
        }

        Ok(SearchUsersRaw {
            items,
            total: total as u64,
        })
    }

    pub fn search_media(&self, req: MediaSearchRequest) -> Result<SearchMediaRaw> {
        let searcher = self.reader.searcher();
        let mut query_clauses: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];

        // Text query on name
        if let Some(q_str) = &req.inner.query {
            if !q_str.is_empty() {
                let query_parser = QueryParser::for_index(searcher.index(), vec![self.schema.name]);

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                query_clauses.push((tantivy::query::Occur::Must, parsed_query));
            }
        }

        // Filter by doctype = Media
        let doctype_term = Term::from_field_text(self.schema.doctype, "Media");
        query_clauses.push((
            tantivy::query::Occur::Must,
            Box::new(tantivy::query::TermQuery::new(
                doctype_term,
                tantivy::schema::IndexRecordOption::Basic,
            )),
        ));

        let query = BooleanQuery::new(query_clauses);

        let limit = req.inner.limit as usize;
        let cursor = req.inner.offset as usize;

        let (top_docs, total) = searcher
            .search(
                &query,
                &(TopDocs::with_limit(limit).and_offset(cursor), Count),
            )
            .map_err(|e| Error::Internal(format!("Search failed: {e}")))?;

        let top_docs: Vec<DocAddress> = top_docs.into_iter().map(|(_, doc)| doc).collect();

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

            items.push(id.parse().unwrap());
        }

        Ok(SearchMediaRaw {
            items,
            total: total as u64,
        })
    }

    pub fn search_audit_log(&self, req: AuditLogSearchRequest) -> Result<SearchAuditLogRaw> {
        let searcher = self.reader.searcher();
        let mut query_clauses: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];

        // Text query on name/message
        if let Some(q_str) = &req.inner.query {
            if !q_str.is_empty() {
                let query_parser = QueryParser::for_index(searcher.index(), vec![self.schema.name]);

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                query_clauses.push((tantivy::query::Occur::Must, parsed_query));
            }
        }

        // Filter by doctype = AuditLog
        let doctype_term = Term::from_field_text(self.schema.doctype, "AuditLog");
        query_clauses.push((
            tantivy::query::Occur::Must,
            Box::new(tantivy::query::TermQuery::new(
                doctype_term,
                tantivy::schema::IndexRecordOption::Basic,
            )),
        ));

        let query = BooleanQuery::new(query_clauses);

        let limit = req.inner.limit as usize;
        let cursor = req.inner.offset as usize;

        let (top_docs, total) = searcher
            .search(
                &query,
                &(TopDocs::with_limit(limit).and_offset(cursor), Count),
            )
            .map_err(|e| Error::Internal(format!("Search failed: {e}")))?;

        let top_docs: Vec<DocAddress> = top_docs.into_iter().map(|(_, doc)| doc).collect();

        let mut items = vec![];
        for doc_address in top_docs {
            let retrieved_doc: TantivyDocument =
                searcher.doc(doc_address).expect("doc fetch failed");

            let id = retrieved_doc
                .get_first(self.schema.id)
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Internal("Document missing id field".into()))?;

            let room_id = retrieved_doc
                .get_first(self.schema.room_id)
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Internal("Document missing room_id field".into()))?;

            items.push((room_id.parse().unwrap(), id.parse().unwrap()));
        }

        Ok(SearchAuditLogRaw {
            items,
            total: total as u64,
        })
    }
}

pub struct SearchUsersRaw {
    pub items: Vec<UserId>,
    pub total: u64,
}

pub struct SearchMediaRaw {
    pub items: Vec<MediaId>,
    pub total: u64,
}

pub struct SearchAuditLogRaw {
    pub items: Vec<(RoomId, AuditLogEntryId)>,
    pub total: u64,
}
