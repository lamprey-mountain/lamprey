use std::{
    path::PathBuf,
    prelude::rust_2024::Future,
    sync::{mpsc, Arc},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use common::v1::types::{
    search::{MessageSearchOrderField, MessageSearchRequest, Order},
    ChannelId, MessageId,
};
use kameo::{
    actor::{ActorRef, Spawn},
    prelude::{Context, Message},
    Actor,
};
use tantivy::{IndexReader, IndexWriter, TantivyDocument, Term};
use tracing::error;

use crate::{
    services::search::{
        directory::ObjectDirectory, import::ImportActor, tokenizer::DynamicTokenizer,
    },
    ServerStateInner,
};

/// buffer size split between indexing threads
///
/// currently set to 100mb
const INDEXING_BUFFER_SIZE: usize = 100_000_000;

/// how frequently to commit the index
const COMMIT_INTERVAL: Duration = Duration::from_secs(5);

/// the maximum of uncommitted documents before needing to commit
const MAX_UNCOMMITTED: usize = 1000;

pub struct IndexManager {
    s: Arc<ServerStateInner>,
}

enum IndexerCommand {
    Add(TantivyDocument),
    Delete(Term),
    Commit,
}

impl IndexManager {
    pub fn new(s: Arc<ServerStateInner>) -> Self {
        Self { s }
    }

    pub fn open(&self, name: &str, schema: &tantivy::schema::Schema) -> ActorRef<IndexActor> {
        let (tx, rx) = mpsc::sync_channel::<IndexerCommand>(1000);
        let (init_tx, init_rx) = mpsc::sync_channel(0);
        let _rt = self.s.tokio.clone();
        let s = Arc::clone(&self.s);
        let name = name.to_owned();
        let schema = schema.to_owned();

        let thread = std::thread::spawn(move || {
            let dir = ObjectDirectory::new(
                s,
                PathBuf::from(format!("tantivy/{name}")),
                PathBuf::from(format!("/tmp/tantivy/{name}")),
            );
            let index = tantivy::Index::open_or_create(dir, schema.to_owned()).unwrap();
            index
                .tokenizers()
                .register("dynamic", DynamicTokenizer::new());

            let reader = index.reader().expect("failed to create index reader");
            init_tx
                .send((index.clone(), schema.clone(), reader))
                .expect("failed to send init data");

            let mut index_writer: IndexWriter = index.writer(INDEXING_BUFFER_SIZE).unwrap();
            let mut last_commit = Instant::now();
            let mut uncommitted_count = 0;

            loop {
                let timeout = if uncommitted_count > 0 {
                    COMMIT_INTERVAL.saturating_sub(last_commit.elapsed())
                } else {
                    Duration::from_secs(60)
                };

                let cmd = if timeout.is_zero() {
                    rx.try_recv().ok()
                } else {
                    rx.recv_timeout(timeout).ok()
                };

                if let Some(cmd) = cmd {
                    match cmd {
                        IndexerCommand::Add(doc) => {
                            if let Err(e) = index_writer.add_document(doc) {
                                error!("failed to add document: {}", e);
                            }
                            uncommitted_count += 1;
                        }
                        IndexerCommand::Delete(term) => {
                            index_writer.delete_term(term);
                            uncommitted_count += 1;
                        }
                        IndexerCommand::Commit => {
                            if let Err(e) = index_writer.commit() {
                                error!("Commit failed: {}", e);
                            }
                        }
                    }
                }

                if uncommitted_count > 0
                    && (uncommitted_count >= MAX_UNCOMMITTED
                        || last_commit.elapsed() >= COMMIT_INTERVAL)
                {
                    if let Err(e) = index_writer.commit() {
                        error!("Commit failed: {}", e);
                    }
                    last_commit = Instant::now();
                    uncommitted_count = 0;
                }
            }
        });

        let (_index, _schema, reader) = init_rx.recv().expect("failed to recv init data");
        IndexActor::spawn(IndexActor { tx, reader, thread })
    }

    pub fn spawn_importer(&self) -> ActorRef<ImportActor> {
        todo!()
    }
}

/// actor representing an index that can be read from or written to
#[derive(Actor)]
pub struct IndexActor {
    tx: std::sync::mpsc::SyncSender<IndexerCommand>,
    reader: IndexReader,
    thread: JoinHandle<()>,
}

pub struct Search {
    pub req: MessageSearchRequest,
    pub visible_channel_ids: Vec<(ChannelId, bool)>,
}

pub struct CommitIndex;

pub struct AddDocument(pub TantivyDocument);

pub struct DeleteTerm(pub Term);

/// actor for querying messages
#[derive(Actor)]
pub struct QueryMessagesActor {
    reader: IndexReader,
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

impl Message<SearchMessages> for QueryMessagesActor {
    type Reply = SearchMessagesResponseRaw;

    async fn handle(
        &mut self,
        msg: SearchMessages,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        use tantivy::{
            collector::{Count, TopDocs},
            query::{BooleanQuery, Query, QueryParser},
            schema::Value,
            DocAddress, TantivyDocument, Term,
        };

        let searcher = self.reader.searcher();
        let mut query_clauses: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];

        // Build query from request
        if let Some(q_str) = &msg.req.query {
            if !q_str.is_empty() {
                // TODO: need index reference for QueryParser
                // For now, skip full-text query building
            }
        }

        // Visibility filter
        let mut vis_queries: Vec<(tantivy::query::Occur, Box<dyn Query>)> = vec![];
        for (id, can_view_private_threads) in &msg.visible_channel_ids {
            vis_queries.push((
                tantivy::query::Occur::Should,
                Box::new(tantivy::query::TermQuery::new(
                    Term::from_field_text(
                        tantivy::schema::Field::from_field_id(0), // TODO: get from schema
                        &id.to_string(),
                    ),
                    tantivy::schema::IndexRecordOption::Basic,
                )),
            ));

            if *can_view_private_threads {
                vis_queries.push((
                    tantivy::query::Occur::Should,
                    Box::new(tantivy::query::TermQuery::new(
                        Term::from_field_text(
                            tantivy::schema::Field::from_field_id(0), // TODO: parent_channel_id
                            &id.to_string(),
                        ),
                        tantivy::schema::IndexRecordOption::Basic,
                    )),
                ));
            }
        }

        if vis_queries.is_empty() {
            return SearchMessagesResponseRaw {
                items: vec![],
                total: 0,
            };
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
                .get_first(tantivy::schema::Field::from_field_id(0))
                .unwrap()
                .as_str()
                .unwrap();
            let channel_id = retrieved_doc
                .get_first(tantivy::schema::Field::from_field_id(0))
                .unwrap()
                .as_str()
                .unwrap();
            items.push(SearchMessagesResponseRawItem {
                id: id.parse().unwrap(),
                channel_id: channel_id.parse().unwrap(),
            });
        }

        SearchMessagesResponseRaw { items, total }
    }
}

/// actor for querying channels
#[derive(Actor)]
pub struct QueryChannelsActor {
    reader: IndexReader,
}

/// actor for querying rooms
#[derive(Actor)]
pub struct QueryRoomsActor {
    reader: IndexReader,
}

/// actor for querying users
#[derive(Actor)]
pub struct QueryUsersActor {
    reader: IndexReader,
}

/// actor for querying room analytics
#[derive(Actor)]
pub struct QueryRoomAnalyticsActor {
    reader: IndexReader,
}

/// actor for querying document history
#[derive(Actor)]
pub struct QueryDocumentHistoryActor {
    reader: IndexReader,
}

impl Message<CommitIndex> for IndexActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: CommitIndex,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Err(e) = self.tx.send(IndexerCommand::Commit) {
            error!("failed to send commit command: {}", e);
        }
    }
}

impl Message<AddDocument> for IndexActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: AddDocument,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Err(e) = self.tx.send(IndexerCommand::Add(msg.0)) {
            error!("failed to send add document command: {}", e);
        }
    }
}

impl Message<DeleteTerm> for IndexActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: DeleteTerm,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Err(e) = self.tx.send(IndexerCommand::Delete(msg.0)) {
            error!("failed to send delete term command: {}", e);
        }
    }
}

impl Message<Search> for IndexActor {
    type Reply = ();

    async fn handle(&mut self, _msg: Search, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        todo!()
    }
}
