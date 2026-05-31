use tantivy::{
    collector::{Count, TopDocs},
    query::{BooleanQuery, Occur, Query, QueryParser, TermQuery},
    schema::IndexRecordOption,
    DocAddress, Term,
};

use common::v1::types::search::{
    ChannelSearchRequest, MessageSearchOrderField, MessageSearchRequest, RoomSearchRequest,
};
use common::v1::types::{ChannelId, RoomId};

use crate::services::search::util::SCHEMA;
use crate::services::search::{
    index::glue::{TantivyChannel, TantivyRoom},
    util::generate_tantivy_query_for_channel_visibility,
};
use crate::services::search::{
    index::{glue::TantivyMessage, AsyncSearcher},
    util::IntoTantivyOrder,
};
use crate::{Error, Result};

/// wrapper around `AsyncSearcher`
pub struct ContentSearcher {
    searcher: AsyncSearcher,
}

pub struct TantivySearchMessages {
    pub req: MessageSearchRequest,
    pub visible_channel_ids: Vec<(ChannelId, bool)>,
}

pub struct TantivySearchChannels {
    pub req: ChannelSearchRequest,
    pub visible_room_ids: Vec<RoomId>,
}

pub struct TantivySearchRooms {
    pub req: RoomSearchRequest,

    /// restrict room search visibilty
    ///
    /// - if Some: only search public rooms + rooms in these
    /// - if None: search everything
    pub restriction: RoomSearchRestriction,
}

pub enum RoomSearchRestriction {
    /// public rooms + these rooms
    Public(Vec<RoomId>),

    /// only public rooms
    PublicOnly,

    /// all rooms
    Everything,
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

impl ContentSearcher {
    pub fn new(searcher: AsyncSearcher) -> Self {
        Self { searcher }
    }

    pub async fn search_messages(&self, msg: TantivySearchMessages) -> Result<TantivyMessages> {
        if msg.visible_channel_ids.is_empty() {
            return Ok(TantivyMessages {
                items: vec![],
                total: 0,
            });
        }

        let mut query_clauses: Vec<(Occur, Box<dyn Query>)> = vec![];

        if let Some(q_str) = &msg.req.inner.query {
            if !q_str.is_empty() {
                let query_parser = QueryParser::for_index(
                    self.searcher.index(),
                    vec![SCHEMA.content, SCHEMA.name],
                );

                let parsed_query = query_parser
                    .parse_query(q_str)
                    .map_err(|e| Error::Internal(format!("Search syntax error: {e}")))?;

                query_clauses.push((Occur::Must, parsed_query));
            }
        }

        let doctype_term = Term::from_field_text(SCHEMA.doctype, "Message");
        query_clauses.push((
            Occur::Must,
            Box::new(TermQuery::new(doctype_term, IndexRecordOption::Basic)),
        ));

        query_clauses.push((
            Occur::Must,
            Box::new(generate_tantivy_query_for_channel_visibility(
                &msg.visible_channel_ids,
            )),
        ));

        let query = BooleanQuery::new(query_clauses);

        let limit = msg.req.inner.limit as usize;
        let cursor = msg.req.inner.offset as usize;

        let (items_raw, count): (Vec<_>, _) = match (msg.req.sort_field, msg.req.inner.sort_order) {
            (MessageSearchOrderField::Relevancy, _) => {
                let top_docs = TopDocs::with_limit(limit).and_offset(cursor);
                let (docs, count): (Vec<(f32, DocAddress)>, usize) =
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
                let (docs, count): (Vec<(tantivy::DateTime, DocAddress)>, usize) =
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

    pub async fn search_channels(&self, req: TantivySearchChannels) -> Result<TantivyChannels> {
        todo!()
    }

    pub async fn search_rooms(&self, req: TantivySearchRooms) -> Result<TantivyRooms> {
        todo!()
    }

    // TODO: search_users
    // TODO: search_media
    // TODO: search_audit_log_entries
}
