use std::{
    path::PathBuf,
    sync::{
        mpsc::{self, Sender},
        Arc,
    },
};

use common::v1::types::{
    search::{MessageSearchOrderField, MessageSearchRequest, Order},
    ChannelId, Message, MessageId, MessageSync, MessageType,
};
use tantivy::{
    collector::{Count, TopDocs},
    query::QueryParser,
    schema::Value,
    DocAddress, Document, Index, IndexWriter, Score, TantivyDocument, Term,
};
use tracing::error;

use crate::{
    services::search::{
        directory::ObjectDirectory,
        schema::{tantivy_document_from_message, LampreySchema},
        tokenizer::DynamicTokenizer,
    },
    Result, ServerState, ServerStateInner,
};

/// buffer size split between indexing threads
///
/// currently set to 100mb
const INDEXING_BUFFER_SIZE: usize = 100_000_000;

pub struct TantivyHandle {
    command_tx: std::sync::mpsc::SyncSender<IndexerCommand>,
    thread: std::thread::JoinHandle<()>,
    index: Index,
    schema: LampreySchema,
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
    let dir = ObjectDirectory::new(s, PathBuf::from("tantivy/"), PathBuf::from("/tmp/tantivy"));
    let sch = LampreySchema::default();
    let index = Index::open_or_create(dir, sch.schema.clone()).unwrap();
    index
        .tokenizers()
        .register("dynamic", DynamicTokenizer::new());
    let (tx, rx) = mpsc::sync_channel::<IndexerCommand>(1000);

    let index2 = index.clone();
    let sch2 = sch.clone();

    let thread = std::thread::spawn(move || {
        let mut index_writer: IndexWriter = index2.writer(INDEXING_BUFFER_SIZE).unwrap();

        let insert_message = |index_writer: &IndexWriter, message: Message| {
            let doc = tantivy_document_from_message(&sch2, message);
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
                                    sch2.id,
                                    &message.id.to_string(),
                                ));
                                insert_message(&index_writer, message);
                            }
                            MessageSync::MessageDelete {
                                channel_id: _,
                                message_id,
                            } => {
                                index_writer.delete_term(Term::from_field_text(
                                    sch2.id,
                                    &message_id.to_string(),
                                ));
                            }
                            MessageSync::MessageDeleteBulk { message_ids, .. } => {
                                for message_id in message_ids {
                                    index_writer.delete_term(Term::from_field_text(
                                        sch2.id,
                                        &message_id.to_string(),
                                    ));
                                }
                            }
                            // TODO: handle Message{Remove,Restore}
                            _ => {}
                        }
                        uncommitted_count += 1;
                    }
                    IndexerCommand::ReindexChannel(channel_id) => {
                        index_writer.delete_term(Term::from_field_text(
                            sch2.channel_id,
                            &channel_id.to_string(),
                        ));
                        // Force commit before potential long operation
                        if let Err(e) = index_writer.commit() {
                            error!("Commit failed: {}", e);
                        }
                        last_commit = std::time::Instant::now();
                        uncommitted_count = 0;

                        // TODO: fetch messages from db, index them into tantivy
                        // TODO: resume when server is restarted
                        todo!()
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
        }

        let _ = index_writer.commit();
    });

    TantivyHandle {
        command_tx: tx,
        thread,
        index,
        schema: sch,
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
    pub fn search_messages(&self, req: MessageSearchRequest) -> Result<SearchMessagesResponseRaw> {
        let reader = self.index.reader()?;
        let s = &self.schema;
        let searcher = reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![s.content]);

        // // maybe use fuzzy search (within levenshein distance)?
        // query_parser.set_field_fuzzy(s.content, false, 3, true);

        // also look at parse_query_lenient
        // TODO: return better error here
        let query = query_parser.parse_query(&req.query.unwrap()).unwrap();

        let limit = 20;
        let cursor = 0;
        let collector = TopDocs::with_limit(limit).and_offset(cursor);
        // FIXME: message ordering
        let top_docs: Vec<DocAddress> = match (req.sort_field, req.sort_order) {
            (MessageSearchOrderField::Relevancy, _) => searcher
                .search(&query, &collector)?
                .into_iter()
                .map(|(_, doc)| doc)
                .collect(),
            (MessageSearchOrderField::Created, Order::Ascending) => searcher
                .search(
                    &query,
                    &collector.order_by_fast_field::<tantivy::DateTime>(
                        "created_at",
                        tantivy::Order::Asc,
                    ),
                )?
                .into_iter()
                .map(|(_, doc)| doc)
                .collect(),
            (MessageSearchOrderField::Created, Order::Descending) => searcher
                .search(
                    &query,
                    &collector.order_by_fast_field::<tantivy::DateTime>(
                        "created_at",
                        tantivy::Order::Desc,
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
            println!("{}", retrieved_doc.to_json(&s.schema));
            dbg!((id, channel_id));
            items.push(SearchMessagesResponseRawItem {
                id: id.parse().unwrap(),
                channel_id: channel_id.parse().unwrap(),
            });
        }

        Ok(SearchMessagesResponseRaw { items, total })
    }
}
