use std::{
    path::PathBuf,
    sync::{mpsc, Arc},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use common::v1::types::{search::MessageSearchRequest, ChannelId};
use kameo::{
    actor::{ActorRef, Spawn},
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

// impl Message<CommitIndex> for IndexerActor {}
// impl Message<AddDocument> for IndexerActor {}
// impl Message<DeleteTerm> for IndexerActor {}
// impl Message<Search> for IndexerActor {}
