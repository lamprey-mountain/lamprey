use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use common::v1::types::{
    search::{MessageSearchOrderField, MessageSearchRequest, Order},
    ChannelId, MessageId,
};
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
        directory::ObjectDirectory,
        import::ImportActor,
        schema::{content::ContentSchema, IndexDefinition},
        tokenizer::DynamicTokenizer,
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

    pub fn spawn_importer(&self) -> ActorRef<ImportActor> {
        todo!()
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

/// actor for querying messages
#[derive(Actor)]
pub struct QueryMessagesActor {
    reader: IndexReader,
    schema: ContentSchema,
}

pub struct SearchMessages {
    pub req: MessageSearchRequest,
    pub visible_channel_ids: Vec<(ChannelId, bool)>,
}

pub struct SearchMessagesResponseRawItem {
    pub id: MessageId,
    pub channel_id: ChannelId,
}

#[derive(kameo::Reply)]
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
            query::{BooleanQuery, Query},
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
