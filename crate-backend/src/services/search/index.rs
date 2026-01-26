use std::{
    path::PathBuf,
    sync::{
        mpsc::{self, Sender},
        Arc,
    },
};

use common::v1::types::{
    search::{SearchMessageOrder, SearchMessageRequest},
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
    services::search::{directory::ObjectDirectory, schema::MessageSchema},
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
    schema: MessageSchema,
}

pub enum IndexerCommand {
    /// handle this event and update
    Message(MessageSync),

    /// reindex all messages in this channel
    ReindexChannel(ChannelId),

    /// commit/flush then exit
    Shutdown,
}

pub fn spawn_indexer(s: Arc<ServerStateInner>) -> TantivyHandle {
    let dir = ObjectDirectory::new(s, PathBuf::from("tantivy/"), PathBuf::from("/tmp/tantivy"));
    let sch = MessageSchema::default();
    let index = Index::open_or_create(dir, sch.schema.clone()).unwrap();
    let (tx, rx) = mpsc::channel::<IndexerCommand>();

    let index2 = index.clone();
    let sch2 = sch.clone();

    let thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async move {
            let mut index_writer: IndexWriter = index2.writer(INDEXING_BUFFER_SIZE).unwrap();

            let insert_message = |index_writer: &IndexWriter, message: Message| {
                let mut doc = TantivyDocument::new();
                doc.add_text(sch2.id, message.id.to_string());
                doc.add_text(sch2.channel_id, message.channel_id.to_string());
                doc.add_text(sch2.author_id, message.author_id.to_string());
                doc.add_date(
                    sch2.created_at,
                    tantivy::DateTime::from_utc(*message.created_at),
                );
                match message.latest_version.message_type {
                    MessageType::DefaultMarkdown(m) => {
                        if let Some(c) = &m.content {
                            doc.add_text(sch2.content, c);
                        }
                        doc.add_bool(sch2.has_attachment, !m.attachments.is_empty());
                        doc.add_bool(
                            sch2.has_image,
                            m.attachments
                                .iter()
                                .any(|a| a.source.mime.starts_with("image/")),
                        );
                        doc.add_bool(
                            sch2.has_audio,
                            m.attachments
                                .iter()
                                .any(|a| a.source.mime.starts_with("audio/")),
                        );
                        doc.add_bool(
                            sch2.has_video,
                            m.attachments
                                .iter()
                                .any(|a| a.source.mime.starts_with("video/")),
                        );
                        doc.add_bool(sch2.has_embed, !m.embeds.is_empty());
                        // doc.add_bool(sch2.has_link, todo);
                        // doc.add_bool(sch2.has_thread, todo);
                    }
                    _ => {}
                }
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

struct SearchMessagesResponseRaw {
    items: Vec<SearchMessagesResponseRawItem>,
    total: u64,
}

struct SearchMessagesResponseRawItem {
    id: MessageId,
    channel_id: ChannelId,
}

impl TantivyHandle {
    pub fn search_messages(&self, req: SearchMessageRequest) -> Result<SearchMessagesResponseRaw> {
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
        let top_docs: Vec<DocAddress> = match req.order {
            SearchMessageOrder::Relevancy => searcher
                .search(&query, &collector)?
                .into_iter()
                .map(|(_, doc)| doc)
                .collect(),
            SearchMessageOrder::Newest => searcher
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
            SearchMessageOrder::Oldest => searcher
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
