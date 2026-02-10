use std::{
    path::PathBuf,
    sync::{
        mpsc::{self, Sender},
        Arc,
    },
};

use common::v1::types::{
    search::{MessageSearchOrderField, MessageSearchRequest, Order}, ChannelId, Message, MessageId, MessageSync, MessageType,
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
    command_tx: Sender<IndexerCommand>,
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
    let (tx, rx) = mpsc::channel::<IndexerCommand>();

    let index2 = index.clone();
    let sch2 = sch.clone();

    let thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        // TODO: better error handling
        rt.block_on(async move {
            let mut index_writer: IndexWriter = index2.writer(INDEXING_BUFFER_SIZE).unwrap();

            let insert_message = |index_writer: &IndexWriter, message: Message| {
                let doc = tantivy_document_from_message(&sch2, message);
                index_writer.add_document(doc).unwrap();
            };

            while let Ok(cmd) = rx.recv() {
                // PERF: don't commit every time, batch commits togeter. maybe throttle commits to every n seconds.
                match cmd {
                    IndexerCommand::Message(msg) => match msg {
                        MessageSync::MessageCreate { message } => {
                            insert_message(&index_writer, message);
                            index_writer.commit().unwrap();
                        }
                        MessageSync::MessageUpdate { message } => {
                            index_writer.delete_term(Term::from_field_text(
                                sch2.id,
                                &message.id.to_string(),
                            ));
                            insert_message(&index_writer, message);
                            index_writer.commit().unwrap();
                        }
                        MessageSync::MessageDelete {
                            channel_id,
                            message_id,
                        } => {
                            index_writer.delete_term(Term::from_field_text(
                                sch2.id,
                                &message_id.to_string(),
                            ));
                            index_writer.commit().unwrap();
                        }
                        MessageSync::MessageDeleteBulk { message_ids, .. } => {
                            for message_id in message_ids {
                                index_writer.delete_term(Term::from_field_text(
                                    sch2.id,
                                    &message_id.to_string(),
                                ));
                            }
                            let _opstamp = index_writer.commit().unwrap();
                            // everything up to opstamp has been successfully written now
                        }
                        // TODO: handle Message{Remove,Restore}
                        _ => {}
                    },
                    IndexerCommand::ReindexChannel(channel_id) => {
                        index_writer.delete_term(Term::from_field_text(
                            sch2.channel_id,
                            &channel_id.to_string(),
                        ));
                        let _opstamp = index_writer.commit().unwrap();
                        // TODO: fetch messages from db, index them into tantivy
                        // TODO: resume when server is restarted
                        todo!()
                    }
                    IndexerCommand::Shutdown => break,
                }
            }

            let _ = index_writer.commit();
        });
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
