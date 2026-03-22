use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use common::v1::types::{search::MessageSearchRequest, ChannelId};
use dashmap::DashMap;
use kameo::{
    actor::{ActorRef, Spawn},
    prelude::{Context, Message},
    Actor,
};
use lamprey_backend_core::prelude::*;
use tantivy::{IndexReader, IndexWriter, TantivyDocument, Term};
use tracing::error;

use crate::{
    services::search::{
        directory::ObjectDirectory, schema::IndexDefinition, tokenizer::DynamicTokenizer,
    },
    ServerStateInner,
};

use super::util::{COMMIT_INTERVAL, INDEXING_BUFFER_SIZE, MAX_UNCOMMITTED};

pub struct IndexManager {
    s: Arc<ServerStateInner>,

    /// Registry to ensure we only open one writer/reader per index name
    registry: DashMap<String, (IndexActorRef, IndexReader)>,
}

pub type IndexActorRef = ActorRef<IndexActor>;

impl IndexManager {
    pub fn new(s: Arc<ServerStateInner>) -> Self {
        Self {
            s,
            registry: DashMap::new(),
        }
    }

    pub async fn open<T: IndexDefinition>(&self, def: T) -> Result<(IndexActorRef, IndexReader)> {
        let name = def.name();

        if let Some(entry) = self.registry.get(&name) {
            return Ok(entry.value().clone());
        }

        let s = Arc::clone(&self.s);
        let schema = def.schema().to_owned();
        let name_clone = name.clone();

        let (reader, writer) = tokio::task::spawn_blocking(move || {
            let dir = ObjectDirectory::new(
                s,
                PathBuf::from(format!("tantivy/{name_clone}")),
                PathBuf::from(format!("/tmp/tantivy/{name_clone}")),
            );

            let index = tantivy::Index::open_or_create(dir, schema)
                .map_err(|e| Error::Internal(format!("Failed to open index: {e}")))?;

            index
                .tokenizers()
                .register("dynamic", DynamicTokenizer::new());

            let reader = index
                .reader()
                .map_err(|e| Error::Internal(format!("Failed to create reader: {e}")))?;

            let writer = index
                .writer(INDEXING_BUFFER_SIZE)
                .map_err(|e| Error::Internal(format!("Failed to create writer: {e}")))?;

            Ok::<(IndexReader, IndexWriter), Error>((reader, writer))
        })
        .await
        .map_err(|e| Error::Internal(format!("Task join error: {e}")))??;

        let actor_ref = IndexActor::spawn(IndexActor::new(writer, reader.clone()));
        let handles = (actor_ref, reader);
        self.registry.insert(name, handles.clone());
        Ok(handles)
    }
}

/// actor representing an index that can be read from or written to
pub struct IndexActor {
    writer: Arc<Mutex<IndexWriter>>,
    reader: IndexReader,
    uncommitted_count: usize,
    last_commit: Instant,
}

impl Actor for IndexActor {
    type Args = IndexActor;
    type Error = Error;

    async fn on_start(args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let _ = actor_ref.tell(CommitIndex).await;
            }
        });

        Ok(args)
    }

    async fn on_stop(
        &mut self,
        _actor_ref: kameo::prelude::WeakActorRef<Self>,
        _reason: kameo::prelude::ActorStopReason,
    ) -> Result<()> {
        let writer = self.writer.clone();
        tokio::task::spawn_blocking(move || {
            let mut writer = writer.lock().unwrap();
            writer
                .commit()
                .expect("Final commit failed during shutdown");
        })
        .await
        .unwrap();
        Ok(())
    }
}

pub struct Search {
    pub req: MessageSearchRequest,
    pub visible_channel_ids: Vec<(ChannelId, bool)>,
}

pub struct CommitIndex;

pub struct AddDocument(pub TantivyDocument);

pub struct DeleteTerm(pub Term);

pub struct UpdateDocument {
    pub term: Term,
    pub doc: TantivyDocument,
}

impl Message<UpdateDocument> for IndexActor {
    type Reply = ();
    async fn handle(&mut self, msg: UpdateDocument, _ctx: &mut Context<Self, Self::Reply>) {
        let writer = self.writer.clone();
        tokio::task::spawn_blocking(move || {
            let writer = writer.lock().unwrap();
            writer.delete_term(msg.term); // Remove old version
            writer.add_document(msg.doc); // Add new version
        })
        .await
        .unwrap();

        self.uncommitted_count += 1;
        self.check_auto_commit().await;
    }
}

impl Message<CommitIndex> for IndexActor {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: CommitIndex,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit().await;
    }
}

impl Message<AddDocument> for IndexActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: AddDocument,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let writer = self.writer.clone();

        tokio::task::spawn_blocking(move || {
            let writer_guard = writer.lock().unwrap();
            if let Err(e) = writer_guard.add_document(msg.0) {
                error!("failed to add document: {}", e);
            }
        })
        .await
        .unwrap();

        self.uncommitted_count += 1;
        self.check_auto_commit().await;
    }
}

impl Message<DeleteTerm> for IndexActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: DeleteTerm,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let writer = self.writer.clone();

        tokio::task::spawn_blocking(move || {
            let writer_guard = writer.lock().unwrap();
            writer_guard.delete_term(msg.0);
        })
        .await
        .unwrap();

        self.uncommitted_count += 1;
        self.check_auto_commit().await;
    }
}

impl Message<Search> for IndexActor {
    type Reply = ();

    async fn handle(&mut self, _msg: Search, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        todo!()
    }
}

impl IndexActor {
    pub fn new(writer: IndexWriter, reader: IndexReader) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
            reader,
            uncommitted_count: 0,
            last_commit: Instant::now(),
        }
    }

    async fn check_auto_commit(&mut self) {
        if self.uncommitted_count > 0
            && (self.uncommitted_count >= MAX_UNCOMMITTED
                || self.last_commit.elapsed() >= COMMIT_INTERVAL)
        {
            self.commit().await;
        }
    }

    async fn commit(&mut self) {
        let writer = self.writer.clone();
        let reader = self.reader.clone();

        let res = tokio::task::spawn_blocking(move || {
            let mut writer_lock = writer.lock().unwrap();
            writer_lock.commit()?;
            // readers MUST be reloaded after commits so searches see new data!
            reader.reload()?;
            Ok::<(), tantivy::TantivyError>(())
        })
        .await;

        match res {
            Ok(Ok(_)) => {
                self.uncommitted_count = 0;
                self.last_commit = Instant::now();
            }
            Ok(Err(e)) => error!("Tantivy commit error: {e}"),
            Err(e) => error!("Blocking task error: {e}"),
        }
    }
}
