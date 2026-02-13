use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{mpsc, Arc},
};

use common::v1::types::{
    search::{MessageSearchOrderField, MessageSearchRequest, Order},
    ChannelId, Message, MessageId, MessageSync, PaginationDirection, PaginationQuery,
    SERVER_USER_ID,
};
use tantivy::{
    collector::{Count, TopDocs},
    query::{BooleanQuery, Query, QueryParser},
    schema::Value,
    DocAddress, Index, IndexWriter, TantivyDocument, Term,
};
use tracing::{debug, error};

use crate::{
    services::search::{
        directory::ObjectDirectory,
        schema::{tantivy_document_from_message, tantivy_document_from_channel, LampreySchema},
        tokenizer::DynamicTokenizer,
    },
    Result, ServerStateInner,
};

/// buffer size split between indexing threads
///
/// currently set to 100mb
const INDEXING_BUFFER_SIZE: usize = 100_000_000;

#[derive(Clone)]
pub struct TantivySearcher {
    pub index: Index,
    pub schema: LampreySchema,
}

pub struct TantivyHandle {
    pub(super) command_tx: std::sync::mpsc::SyncSender<IndexerCommand>,
    #[allow(unused)] // TEMP
    pub(super) thread: std::thread::JoinHandle<()>,
    pub(super) index: Index,
    pub(super) schema: LampreySchema,
    pub(super) searcher: TantivySearcher,
}

pub enum IndexerCommand {
    /// handle this event and update
    Message(MessageSync),

    /// reindex all messages in this channel
    // TODO: save index status? (eg. last indexed message id per channel)
    ReindexChannel(ChannelId),

    /// commit/flush then exit
    Shutdown,
}

/// create a new TantivyHandle
pub fn spawn_indexer(s: Arc<ServerStateInner>) -> TantivyHandle {
    let (tx, rx) = mpsc::sync_channel::<IndexerCommand>(1000);
    let (init_tx, init_rx) = mpsc::sync_channel(0);

    let thread = std::thread::spawn(move || {
        let rt = s.tokio.clone();
        let dir = ObjectDirectory::new(
            Arc::clone(&s),
            PathBuf::from("tantivy/"),
            PathBuf::from("/tmp/tantivy"),
        );
        let sch = LampreySchema::default();
        let index = Index::open_or_create(dir, sch.schema.clone()).unwrap();
        index
            .tokenizers()
            .register("dynamic", DynamicTokenizer::new());

        init_tx
            .send((index.clone(), sch.clone()))
            .expect("failed to send init data");

        let mut index_writer: IndexWriter = index.writer(INDEXING_BUFFER_SIZE).unwrap();

        let insert_message = |index_writer: &IndexWriter, message: Message| {
            // TODO: add message.room_id
            let (room_id, parent_channel_id) = rt.block_on(async {
                if let Ok(channel) = s.services().channels.get(message.channel_id, None).await {
                    (channel.room_id, channel.parent_id)
                } else {
                    (None, None)
                }
            });

            let doc = tantivy_document_from_message(&sch, message, room_id, parent_channel_id);
            if let Err(e) = index_writer.add_document(doc) {
                error!("failed to add document: {}", e);
            }
        };

        let mut last_commit = std::time::Instant::now();
        let mut uncommitted_count = 0;
        const COMMIT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);
        const MAX_UNCOMMITTED: usize = 1000;

        loop {
            let timeout = if uncommitted_count > 0 {
                COMMIT_INTERVAL.saturating_sub(last_commit.elapsed())
            } else {
                std::time::Duration::from_secs(5)
            };

            // If we are overdue, poll immediately (small timeout)
            let timeout = if timeout.is_zero() {
                std::time::Duration::from_millis(1)
            } else {
                timeout
            };

            let cmd = match rx.recv_timeout(timeout) {
                Ok(cmd) => Some(cmd),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            };

            if let Some(cmd) = cmd {
                match cmd {
                    IndexerCommand::Message(msg) => {
                        match msg {
                            MessageSync::MessageCreate { message } => {
                                insert_message(&index_writer, message);
                            }
                            MessageSync::MessageUpdate { message } => {
                                index_writer.delete_term(Term::from_field_text(
                                    sch.id,
                                    &message.id.to_string(),
                                ));
                                insert_message(&index_writer, message);
                            }
                            MessageSync::MessageDelete {
                                channel_id: _,
                                message_id,
                            } => {
                                index_writer.delete_term(Term::from_field_text(
                                    sch.id,
                                    &message_id.to_string(),
                                ));
                            }
                            MessageSync::MessageDeleteBulk { message_ids, .. } => {
                                for message_id in message_ids {
                                    index_writer.delete_term(Term::from_field_text(
                                        sch.id,
                                        &message_id.to_string(),
                                    ));
                                }
                            }
                            MessageSync::ChannelCreate { channel } => {
                                let doc = tantivy_document_from_channel(&sch, *channel);
                                if let Err(e) = index_writer.add_document(doc) {
                                    error!("failed to add channel document: {}", e);
                                }
                            }
                            MessageSync::ChannelUpdate { channel } => {
                                index_writer.delete_term(Term::from_field_text(
                                    sch.id,
                                    &channel.id.to_string(),
                                ));
                                let doc = tantivy_document_from_channel(&sch, *channel);
                                if let Err(e) = index_writer.add_document(doc) {
                                    error!("failed to update channel document: {}", e);
                                }
                            }
                            // TODO: handle Message{Remove,Restore}
                            _ => {}
                        }
                        uncommitted_count += 1;
                    }
                    IndexerCommand::ReindexChannel(channel_id) => {
                        index_writer.delete_term(Term::from_field_text(
                            sch.channel_id,
                            &channel_id.to_string(),
                        ));
                        // Force commit before potential long operation
                        if let Err(e) = index_writer.commit() {
                            error!("Commit failed: {}", e);
                        }
                        last_commit = std::time::Instant::now();
                        uncommitted_count = 0;

                        if let Err(e) =
                            rt.block_on(s.data().search_reindex_queue_upsert(channel_id, None))
                        {
                            error!("Failed to upsert reindex queue: {}", e);
                        }
                    }
                    IndexerCommand::Shutdown => break,
                }
            }

            if uncommitted_count > 0
                && (uncommitted_count >= MAX_UNCOMMITTED
                    || last_commit.elapsed() >= COMMIT_INTERVAL)
            {
                if let Err(e) = index_writer.commit() {
                    error!("Commit failed: {}", e);
                }
                last_commit = std::time::Instant::now();
                uncommitted_count = 0;
            }

            // process reindex queue
            let queue_result = rt.block_on(s.data().search_reindex_queue_list(1));
            match queue_result {
                Ok(items) => {
                    if let Some((channel_id, last_message_id)) = items.first() {
                        let limit = 100;
                        debug!(
                            "reindexing channel {} from {:?}",
                            channel_id, last_message_id
                        );
                        let res = rt.block_on(s.services().messages.list_all(
                            *channel_id,
                            SERVER_USER_ID,
                            PaginationQuery {
                                from: *last_message_id,
                                to: None,
                                dir: Some(PaginationDirection::F),
                                limit: Some(limit),
                            },
                        ));

                        match res {
                            Ok(page) => {
                                if page.items.is_empty() {
                                    // finished reindexing this channel!
                                    if let Err(e) = rt
                                        .block_on(s.data().search_reindex_queue_delete(*channel_id))
                                    {
                                        error!("failed to delete from reindex queue: {}", e);
                                    }
                                } else {
                                    let last_id = page.items.last().map(|m| m.id);
                                    for msg_v2 in page.items {
                                        let msg: Message = msg_v2.into();
                                        insert_message(&index_writer, msg);
                                    }
                                    uncommitted_count += limit as usize;

                                    if let Some(lid) = last_id {
                                        if let Err(e) =
                                            rt.block_on(s.data().search_reindex_queue_upsert(
                                                *channel_id,
                                                Some(lid),
                                            ))
                                        {
                                            error!("failed to update reindex queue: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("failed to list messages for reindex: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("failed to retrieve reindex queue: {}", e);
                }
            }
        }

        let _ = index_writer.commit();
    });

    let (index, schema) = init_rx.recv().expect("failed to recv init data");

    let searcher = TantivySearcher {
        index: index.clone(),
        schema: schema.clone(),
    };

    TantivyHandle {
        command_tx: tx,
        thread,
        index,
        schema,
        searcher,
    }
}

pub struct SearchMessagesResponseRaw {
    pub items: Vec<SearchMessagesResponseRawItem>,
    pub total: u64,
}

pub struct SearchMessagesResponseRawItem {
    pub id: MessageId,
    pub channel_id: ChannelId,
}

impl TantivyHandle {
    pub fn searcher(&self) -> TantivySearcher {
        self.searcher.clone()
    }
}

impl TantivySearcher {
    pub fn search_messages(
        &self,
        req: MessageSearchRequest,
        visible_channel_ids: &[(ChannelId, bool)],
    ) -> Result<SearchMessagesResponseRaw> {
        let reader = self.index.reader()?;
        let s = &self.schema;
        let searcher = reader.searcher();

        let mut query_clauses: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];

        if let Some(q_str) = &req.query {
            if !q_str.is_empty() {
                let mut query_parser = QueryParser::for_index(&self.index, vec![s.content, s.name]);
                query_parser.set_field_boost(s.name, 1.5);
                let q = query_parser
                    .parse_query(q_str)
                    .map_err(|e| tantivy::TantivyError::from(e))?;
                query_clauses.push((tantivy::query::Occur::Must, q));
            }
        }

        // Visibility filter:
        // (channel_id IS visible) OR (parent_channel_id IS visible_parent)
        let mut vis_queries: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];
        for (id, can_view_private_threads) in visible_channel_ids {
            vis_queries.push((
                tantivy::query::Occur::Should,
                Box::new(tantivy::query::TermQuery::new(
                    Term::from_field_text(s.channel_id, &id.to_string()),
                    tantivy::schema::IndexRecordOption::Basic,
                )),
            ));

            if *can_view_private_threads {
                vis_queries.push((
                    tantivy::query::Occur::Should,
                    Box::new(tantivy::query::TermQuery::new(
                        Term::from_field_text(s.parent_channel_id, &id.to_string()),
                        tantivy::schema::IndexRecordOption::Basic,
                    )),
                ));
            }
        }

        if vis_queries.is_empty() {
            return Ok(SearchMessagesResponseRaw {
                items: vec![],
                total: 0,
            });
        }

        query_clauses.push((
            tantivy::query::Occur::Must,
            Box::new(BooleanQuery::new(vis_queries)),
        ));

        // User requested channel filter
        if !req.channel_id.is_empty() {
            let mut chan_queries: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];
            for id in &req.channel_id {
                chan_queries.push((
                    tantivy::query::Occur::Should,
                    Box::new(tantivy::query::TermQuery::new(
                        Term::from_field_text(s.channel_id, &id.to_string()),
                        tantivy::schema::IndexRecordOption::Basic,
                    )),
                ));
            }
            query_clauses.push((
                tantivy::query::Occur::Must,
                Box::new(BooleanQuery::new(chan_queries)),
            ));
        }

        let query = BooleanQuery::new(query_clauses);

        let limit = req.limit as usize;
        let cursor = req.offset as usize;
        let collector = TopDocs::with_limit(limit).and_offset(cursor);

        let top_docs: Vec<DocAddress> = match (req.sort_field, req.sort_order) {
            (MessageSearchOrderField::Relevancy, _) => searcher
                .search(&query, &collector)?
                .into_iter()
                .map(|(_, doc)| doc)
                .collect(),
            (MessageSearchOrderField::Created, ord) => searcher
                .search(
                    &query,
                    &collector.order_by_fast_field::<tantivy::DateTime>(
                        "created_at",
                        match ord {
                            Order::Ascending => tantivy::Order::Asc,
                            Order::Descending => tantivy::Order::Desc,
                        },
                    ),
                )?
                .into_iter()
                .map(|(_, doc)| doc)
                .collect(),
        };

        let total = searcher.search(&query, &Count)? as u64;
        let mut items = vec![];

        for doc_address in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            let id = retrieved_doc.get_first(s.id).unwrap().as_str().unwrap();
            let channel_id = retrieved_doc
                .get_first(s.channel_id)
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
