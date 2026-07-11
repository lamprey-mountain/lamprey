use lamprey_backend_core::types::search::Doctype;
use tantivy::{
    DocAddress, Score,
    collector::{Count, TopDocs},
    query::QueryParser,
};

use common::v1::types::search::{
    AuditLogSearchRequest, ChannelSearchOrderField, ChannelSearchRequest, MediaSearchRequest,
    MessageSearchOrderField, MessageSearchRequest, RoomSearchRequest, UserSearchRequest,
};

use crate::services::search::util::SCHEMA;
use crate::services::search::util::visibility::{
    SearchAuditLogVisibility, SearchChannelsVisibility, SearchMediaVisibility,
    SearchMessagesVisibility, SearchRoomsVisibility, TantivyVisibility,
};
use crate::services::search::{
    index::glue::{TantivyAuditLogEntry, TantivyChannel, TantivyMedia, TantivyRoom, TantivyUser},
    util::BqBuilder,
};
use crate::services::search::{
    index::{AsyncSearcher, glue::TantivyMessage},
    util::IntoTantivyOrder,
};
use crate::{Error, Result};

/// wrapper around `AsyncSearcher`
pub struct ContentSearcher {
    searcher: AsyncSearcher,
}

pub struct TantivySearchMessages {
    pub req: MessageSearchRequest,
    pub visibility: SearchMessagesVisibility,
}

pub struct TantivySearchChannels {
    pub req: ChannelSearchRequest,
    pub visibility: SearchChannelsVisibility,
}

pub struct TantivySearchRooms {
    pub req: RoomSearchRequest,
    pub visibility: SearchRoomsVisibility,
}

pub struct TantivySearchUsers {
    pub req: UserSearchRequest,
}

pub struct TantivySearchMedia {
    pub req: MediaSearchRequest,
    pub visibility: SearchMediaVisibility,
}

pub struct TantivySearchAuditLogEntries {
    pub req: AuditLogSearchRequest,
    pub visibility: SearchAuditLogVisibility,
}

pub struct TantivyMessages {
    pub items: Vec<TantivyMessage>,
    pub total: u64,
}

pub struct TantivyChannels {
    pub items: Vec<TantivyChannel>,
    pub total: u64,
}

pub struct TantivyRooms {
    pub items: Vec<TantivyRoom>,
    pub total: u64,
}

pub struct TantivyUsers {
    pub items: Vec<TantivyUser>,
    pub total: u64,
}

pub struct TantivyMediaItems {
    pub items: Vec<TantivyMedia>,
    pub total: u64,
}

pub struct TantivyAuditLogEntries {
    pub items: Vec<TantivyAuditLogEntry>,
    pub total: u64,
}

impl ContentSearcher {
    pub fn new(searcher: AsyncSearcher) -> Self {
        Self { searcher }
    }

    pub async fn search_messages(&self, msg: TantivySearchMessages) -> Result<TantivyMessages> {
        let mut q = BqBuilder::new();

        if let Some(q_str) = &msg.req.inner.query {
            if !q_str.is_empty() {
                let query_parser = QueryParser::for_index(
                    self.searcher.index(),
                    vec![SCHEMA.content, SCHEMA.name],
                );

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                q.must(parsed_query);
            }
        }

        q.must(SCHEMA.query_doctype(Doctype::Message));
        q.must(msg.visibility.into_query());
        let query = q.build();

        let limit = msg.req.inner.limit as usize;
        let cursor = msg.req.inner.offset as usize;

        let (items_raw, count): (Vec<_>, _) = match (msg.req.sort_field, msg.req.inner.sort_order) {
            (MessageSearchOrderField::Relevancy, _) => {
                let top_docs = TopDocs::with_limit(limit)
                    .and_offset(cursor)
                    .order_by_score();
                let (docs, count): (Vec<(Score, DocAddress)>, usize) =
                    self.searcher.search(&query, &(top_docs, Count)).await?;
                (
                    docs.into_iter().map(|(_, addr)| addr).collect(),
                    count as u64,
                )
            }
            (MessageSearchOrderField::Created, ord) => {
                let top_docs = TopDocs::with_limit(limit)
                    .and_offset(cursor)
                    .order_by_fast_field::<tantivy::DateTime>("created_at", ord.tantivy());
                let (docs, count): (Vec<(Option<tantivy::DateTime>, DocAddress)>, usize) =
                    self.searcher.search(&query, &(top_docs, Count)).await?;
                (
                    docs.into_iter().map(|(_, addr)| addr).collect(),
                    count as u64,
                )
            }
        };

        let mut items = Vec::with_capacity(items_raw.len());
        for doc_address in items_raw {
            items.push(self.searcher.doc(doc_address).await?);
        }

        Ok(TantivyMessages {
            items,
            total: count,
        })
    }

    pub async fn search_channels(&self, msg: TantivySearchChannels) -> Result<TantivyChannels> {
        let mut q = BqBuilder::new();

        // Text query on name and content (description)
        if let Some(q_str) = &msg.req.inner.query {
            if !q_str.is_empty() {
                let mut query_parser = QueryParser::for_index(
                    self.searcher.index(),
                    vec![SCHEMA.content, SCHEMA.name],
                );
                query_parser.set_field_boost(SCHEMA.name, 2.0);

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                q.must(parsed_query);
            }
        }

        q.must(SCHEMA.query_doctype(Doctype::Channel));
        q.must(msg.visibility.into_query());
        let query = q.build();

        let limit = msg.req.inner.limit as usize;
        let cursor = msg.req.inner.offset as usize;

        let (items_raw, count): (Vec<_>, _) = match (msg.req.sort_field, msg.req.inner.sort_order) {
            (ChannelSearchOrderField::Relevancy, _) => {
                let top_docs = TopDocs::with_limit(limit)
                    .and_offset(cursor)
                    .order_by_score();
                let (docs, count): (Vec<(Score, DocAddress)>, usize) =
                    self.searcher.search(&query, &(top_docs, Count)).await?;
                (
                    docs.into_iter().map(|(_, addr)| addr).collect(),
                    count as u64,
                )
            }
            (ChannelSearchOrderField::Created, ord) => {
                let top_docs = TopDocs::with_limit(limit)
                    .and_offset(cursor)
                    .order_by_fast_field::<tantivy::DateTime>("created_at", ord.tantivy());
                let (docs, count): (Vec<(Option<tantivy::DateTime>, DocAddress)>, usize) =
                    self.searcher.search(&query, &(top_docs, Count)).await?;
                (
                    docs.into_iter().map(|(_, addr)| addr).collect(),
                    count as u64,
                )
            }
            (ChannelSearchOrderField::Archived, ord) => {
                let top_docs = TopDocs::with_limit(limit)
                    .and_offset(cursor)
                    .order_by_fast_field::<tantivy::DateTime>("archived_at", ord.tantivy());
                let (docs, count): (Vec<(Option<tantivy::DateTime>, DocAddress)>, usize) =
                    self.searcher.search(&query, &(top_docs, Count)).await?;
                (
                    docs.into_iter().map(|(_, addr)| addr).collect(),
                    count as u64,
                )
            }
            (ChannelSearchOrderField::Activity, ord) => {
                let top_docs = TopDocs::with_limit(limit)
                    .and_offset(cursor)
                    .order_by_fast_field::<tantivy::DateTime>("archived_at", ord.tantivy());
                let (docs, count): (Vec<(Option<tantivy::DateTime>, DocAddress)>, usize) =
                    self.searcher.search(&query, &(top_docs, Count)).await?;
                (
                    docs.into_iter().map(|(_, addr)| addr).collect(),
                    count as u64,
                )
            }
            (ChannelSearchOrderField::Name, ord) => {
                let top_docs = TopDocs::with_limit(limit)
                    .and_offset(cursor)
                    .order_by_string_fast_field("name", ord.tantivy());
                let (docs, count): (Vec<(Option<String>, DocAddress)>, usize) =
                    self.searcher.search(&query, &(top_docs, Count)).await?;
                (
                    docs.into_iter().map(|(_, addr)| addr).collect(),
                    count as u64,
                )
            }
            (ChannelSearchOrderField::Id, ord) => {
                let top_docs = TopDocs::with_limit(limit)
                    .and_offset(cursor)
                    .order_by_string_fast_field("id", ord.tantivy());
                let (docs, count): (Vec<(Option<String>, DocAddress)>, usize) =
                    self.searcher.search(&query, &(top_docs, Count)).await?;
                (
                    docs.into_iter().map(|(_, addr)| addr).collect(),
                    count as u64,
                )
            }
            // (ChannelSearchOrderField::Score, ord) => todo!()
            // (ChannelSearchOrderField::Reactions { reaction }, ord) => todo!(),
            _ => todo!(),
        };

        let mut items = Vec::with_capacity(items_raw.len());
        for doc_address in items_raw {
            items.push(self.searcher.doc(doc_address).await?);
        }

        Ok(TantivyChannels {
            items,
            total: count,
        })
    }

    pub async fn search_rooms(&self, msg: TantivySearchRooms) -> Result<TantivyRooms> {
        let mut q = BqBuilder::new();

        if let Some(q_str) = &msg.req.inner.query {
            if !q_str.is_empty() {
                let query_parser = QueryParser::for_index(self.searcher.index(), vec![SCHEMA.name]);

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                q.must(parsed_query);
            }
        }

        q.must(SCHEMA.query_doctype(Doctype::Room));
        q.must(msg.visibility.into_query());
        let query = q.build();

        let limit = msg.req.inner.limit as usize;
        let cursor = msg.req.inner.offset as usize;

        // TODO: handle requested sorting/order
        let top_docs = TopDocs::with_limit(limit)
            .and_offset(cursor)
            .order_by_score();
        let (docs, count): (Vec<(Score, DocAddress)>, usize) =
            self.searcher.search(&query, &(top_docs, Count)).await?;
        let items_raw: Vec<DocAddress> = docs.into_iter().map(|(_, addr)| addr).collect();

        let mut items = Vec::with_capacity(items_raw.len());
        for doc_address in items_raw {
            items.push(self.searcher.doc(doc_address).await?);
        }

        Ok(TantivyRooms {
            items,
            total: count as u64,
        })
    }

    pub async fn search_users(&self, msg: TantivySearchUsers) -> Result<TantivyUsers> {
        let mut q = BqBuilder::new();

        // Text query on name
        if let Some(q_str) = &msg.req.inner.query {
            if !q_str.is_empty() {
                let query_parser = QueryParser::for_index(self.searcher.index(), vec![SCHEMA.name]);

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                q.must(parsed_query);
            }
        }

        q.must(SCHEMA.query_doctype(Doctype::User));
        let query = q.build();

        let limit = msg.req.inner.limit as usize;
        let cursor = msg.req.inner.offset as usize;

        // TODO: handle requested sorting/order
        let top_docs = TopDocs::with_limit(limit)
            .and_offset(cursor)
            .order_by_score();
        let (docs, count): (Vec<(Score, DocAddress)>, usize) =
            self.searcher.search(&query, &(top_docs, Count)).await?;
        let items_raw: Vec<DocAddress> = docs.into_iter().map(|(_, addr)| addr).collect();

        let mut items = Vec::with_capacity(items_raw.len());
        for doc_address in items_raw {
            items.push(self.searcher.doc(doc_address).await?);
        }

        Ok(TantivyUsers {
            items,
            total: count as u64,
        })
    }

    pub async fn search_media(&self, msg: TantivySearchMedia) -> Result<TantivyMediaItems> {
        let mut q = BqBuilder::new();

        // Text query on name/content
        if let Some(q_str) = &msg.req.inner.query {
            if !q_str.is_empty() {
                let query_parser = QueryParser::for_index(
                    self.searcher.index(),
                    vec![SCHEMA.name, SCHEMA.content],
                );

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                q.must(parsed_query);
            }
        }

        q.must(SCHEMA.query_doctype(Doctype::Media));
        q.must(msg.visibility.into_query());
        let query = q.build();

        let limit = msg.req.inner.limit as usize;
        let cursor = msg.req.inner.offset as usize;

        // TODO: handle requested sorting/order
        let top_docs = TopDocs::with_limit(limit)
            .and_offset(cursor)
            .order_by_score();
        let (docs, count): (Vec<(Score, DocAddress)>, usize) =
            self.searcher.search(&query, &(top_docs, Count)).await?;
        let items_raw: Vec<DocAddress> = docs.into_iter().map(|(_, addr)| addr).collect();

        let mut items = Vec::with_capacity(items_raw.len());
        for doc_address in items_raw {
            items.push(self.searcher.doc(doc_address).await?);
        }

        Ok(TantivyMediaItems {
            items,
            total: count as u64,
        })
    }

    pub async fn search_audit_log_entries(
        &self,
        msg: TantivySearchAuditLogEntries,
    ) -> Result<TantivyAuditLogEntries> {
        let mut q = BqBuilder::new();

        // Text query on name
        if let Some(q_str) = &msg.req.inner.query {
            if !q_str.is_empty() {
                let query_parser = QueryParser::for_index(self.searcher.index(), vec![SCHEMA.name]);

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                q.must(parsed_query);
            }
        }

        q.must(SCHEMA.query_doctype(Doctype::AuditLogEntry));
        q.must(msg.visibility.into_query());
        let query = q.build();

        let limit = msg.req.inner.limit as usize;
        let cursor = msg.req.inner.offset as usize;

        // TODO: handle requested sorting/order
        let top_docs = TopDocs::with_limit(limit)
            .and_offset(cursor)
            .order_by_score();
        let (docs, count): (Vec<(Score, DocAddress)>, usize) =
            self.searcher.search(&query, &(top_docs, Count)).await?;
        let items_raw: Vec<DocAddress> = docs.into_iter().map(|(_, addr)| addr).collect();

        let mut items = Vec::with_capacity(items_raw.len());
        for doc_address in items_raw {
            items.push(self.searcher.doc(doc_address).await?);
        }

        Ok(TantivyAuditLogEntries {
            items,
            total: count as u64,
        })
    }
}
