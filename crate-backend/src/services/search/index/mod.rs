use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use tantivy::{
    DocAddress, Index, IndexReader, IndexWriter, Searcher, TantivyDocument, Term,
    collector::Collector, query::Query, schema::document::DocumentDeserialize,
    space_usage::SearcherSpaceUsage,
};
use tokio::sync::{mpsc, oneshot};
use tracing::error;

use crate::{
    Error, Result, ServerStateInner,
    services::search::{
        directory::ObjectDirectory, schema::IndexDefinition, tokenizer::DynamicTokenizer,
    },
};

pub mod glue;
pub mod searcher;

/// a handle for interacting with an AsyncIndex
#[derive(Clone)]
pub struct AsyncIndexHandle {
    chan: mpsc::Sender<Message>,
}

/// a tantivy index, wrapped with tokio
pub struct AsyncIndex {
    index: Index,
    writer: IndexWriter,
    reader: IndexReader,
    chan: mpsc::Receiver<Message>,

    uncommitted_count: u64,
    last_commit: Instant,
    max_uncommitted: usize,
    commit_interval: Duration,
}

/// a wrapper around tantivy to enable asynchronous searching
pub struct AsyncSearcher {
    searcher: Searcher,
}

enum Message {
    CommitIndex(oneshot::Sender<Result<()>>),
    LazyCommit(oneshot::Sender<Result<()>>),
    // NOTE: this is unused, maybe remove? only updates (upserts) are used currently.
    AddDocument(TantivyDocument, oneshot::Sender<Result<()>>),
    DeleteTerm(Term, oneshot::Sender<Result<()>>),
    DeleteAllDocuments(oneshot::Sender<Result<()>>),
    UpdateDocument {
        term: Term,
        doc: TantivyDocument,
        resp: oneshot::Sender<Result<()>>,
    },
    UpdateDocuments(Vec<(Term, TantivyDocument)>, oneshot::Sender<Result<()>>),
    GetSearcher(oneshot::Sender<Result<AsyncSearcher>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShouldCommit {
    /// don't commit the index
    No,

    /// commit the index if not lazy
    IfNeeded,

    /// always commit the index
    Yes,
}

impl AsyncIndexHandle {
    async fn send_op(&self, op: Message) -> Result<()> {
        self.chan
            .send(op)
            .await
            .map_err(|_| Error::Internal("AsyncIndex.chan closed".to_string()))
    }

    /// commit the index
    ///
    /// immedietely tries to commit and flush changes to disk
    pub async fn commit(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_op(Message::CommitIndex(tx)).await?;
        rx.await
            .map_err(|_| Error::Internal("failed to receive response".to_string()))?
    }

    /// lazily commit the index
    ///
    /// commits if the indexing buffer is sufficiently full
    pub async fn lazy_commit(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_op(Message::LazyCommit(tx)).await?;
        rx.await
            .map_err(|_| Error::Internal("failed to receive response".to_string()))?
    }

    pub async fn add_document(&self, doc: TantivyDocument) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_op(Message::AddDocument(doc, tx)).await?;
        rx.await
            .map_err(|_| Error::Internal("failed to receive response".to_string()))?
    }

    pub async fn delete_term(&self, term: Term) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_op(Message::DeleteTerm(term, tx)).await?;
        rx.await
            .map_err(|_| Error::Internal("failed to receive response".to_string()))?
    }

    pub async fn delete_all_documents(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_op(Message::DeleteAllDocuments(tx)).await?;
        rx.await
            .map_err(|_| Error::Internal("failed to receive response".to_string()))?
    }

    pub async fn update_document(&self, term: Term, doc: TantivyDocument) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_op(Message::UpdateDocument {
            term,
            doc,
            resp: tx,
        })
        .await?;
        rx.await
            .map_err(|_| Error::Internal("failed to receive response".to_string()))?
    }

    pub async fn update_documents(&self, updates: Vec<(Term, TantivyDocument)>) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_op(Message::UpdateDocuments(updates, tx)).await?;
        rx.await
            .map_err(|_| Error::Internal("failed to receive response".to_string()))?
    }

    pub async fn shutdown(&self) -> Result<()> {
        // FIXME: actually shut down index
        Ok(())
    }

    /// get an `AsyncSearcher` for this index
    pub async fn searcher(&self) -> Result<AsyncSearcher> {
        let (tx, rx) = oneshot::channel();
        self.send_op(Message::GetSearcher(tx)).await?;
        rx.await
            .map_err(|_| Error::Internal("failed to receive response".to_string()))?
    }
}

impl AsyncIndex {
    pub async fn open<T: IndexDefinition>(
        s: Arc<ServerStateInner>,
        def: T,
    ) -> Result<AsyncIndexHandle> {
        let name_clone = def.name();
        let schema = def.schema().to_owned();
        let config = s.config.search.clone();

        let (index, writer, reader) = tokio::task::spawn_blocking(move || {
            let cache_path = s.config.search.cache_dir
                .clone()
                .map(|p| p.join(&name_clone))
                .unwrap_or_else(|| PathBuf::from(format!("/tmp/tantivy/{name_clone}")));

            let dir = ObjectDirectory::new(
                s,
                PathBuf::from(format!("tantivy/{name_clone}")),
                cache_path,
            );

            let index = Index::open_or_create(dir, schema)
                .map_err(|e| Error::Internal(format!("Failed to open index: {e}")))?;

            index
                .tokenizers()
                .register("dynamic", DynamicTokenizer::new());

            let reader = index
                .reader()
                .map_err(|e| Error::Internal(format!("Failed to create reader: {e}")))?;

            let writer = index
                .writer(config.indexing_buffer_size)
                .map_err(|e| Error::Internal(format!("Failed to create writer: {e}")))?;

            Ok::<_, Error>((index, writer, reader))
        })
        .await
        .map_err(|e| Error::Internal(format!("Task join error: {e}")))??;

        let (tx, rx) = mpsc::channel(1024);

        let me = Self {
            index,
            writer,
            reader,
            chan: rx,
            uncommitted_count: 0,
            last_commit: Instant::now(),
            max_uncommitted: config.max_uncommitted,
            commit_interval: Duration::from_secs(config.commit_interval),
        };

        tokio::task::spawn_blocking(move || me.spawn());

        let handle = AsyncIndexHandle { chan: tx };

        let handle_clone = handle.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                if handle_clone.lazy_commit().await.is_err() {
                    break;
                }
            }
        });

        Ok(handle)
    }

    fn spawn(mut self) {
        while let Some(op) = self.chan.blocking_recv() {
            match op {
                Message::CommitIndex(resp) => _ = resp.send(self.commit()),
                Message::LazyCommit(resp) => {
                    if self.should_commit() == ShouldCommit::Yes {
                        _ = resp.send(self.commit());
                    } else {
                        _ = resp.send(Ok(()));
                    }
                }
                Message::AddDocument(doc, resp) => _ = resp.send(self.add_document(doc)),
                Message::DeleteTerm(term, resp) => _ = resp.send(self.delete_term(term)),
                Message::DeleteAllDocuments(resp) => _ = resp.send(self.delete_all_documents()),
                Message::UpdateDocument { term, doc, resp } => {
                    _ = resp.send(self.update_document(term, doc))
                }
                Message::UpdateDocuments(updates, resp) => {
                    _ = resp.send(self.update_documents(updates))
                }
                Message::GetSearcher(resp) => {
                    let searcher = self.reader.searcher();
                    _ = resp.send(Ok(AsyncSearcher { searcher }));
                }
            }
        }

        if self.uncommitted_count > 0 {
            if let Err(e) = self.commit() {
                error!("failed to commit index on shutdown: {e}");
            }
        }
    }

    fn should_commit(&self) -> ShouldCommit {
        if self.uncommitted_count == 0 {
            return ShouldCommit::No;
        }

        if self.uncommitted_count >= self.max_uncommitted as u64
            || self.last_commit.elapsed() >= self.commit_interval
        {
            return ShouldCommit::Yes;
        }

        ShouldCommit::IfNeeded
    }

    fn commit(&mut self) -> Result<()> {
        self.writer.commit()?;
        self.reader.reload()?;
        self.uncommitted_count = 0;
        self.last_commit = Instant::now();
        Ok(())
    }

    fn add_document(&mut self, doc: TantivyDocument) -> Result<()> {
        self.writer.add_document(doc)?;
        self.uncommitted_count += 1;
        Ok(())
    }

    fn delete_term(&mut self, term: Term) -> Result<()> {
        self.writer.delete_term(term);
        self.uncommitted_count += 1;
        Ok(())
    }

    fn delete_all_documents(&mut self) -> Result<()> {
        self.writer.delete_all_documents()?;
        self.uncommitted_count += 1;
        Ok(())
    }

    fn update_document(&mut self, term: Term, doc: TantivyDocument) -> Result<()> {
        self.writer.delete_term(term);
        self.writer.add_document(doc)?;
        self.uncommitted_count += 1;
        Ok(())
    }

    fn update_documents(&mut self, updates: Vec<(Term, TantivyDocument)>) -> Result<()> {
        for (term, doc) in updates {
            self.writer.delete_term(term);
            self.writer.add_document(doc)?;
            self.uncommitted_count += 1;
        }
        Ok(())
    }
}

impl AsyncSearcher {
    pub fn index(&self) -> &Index {
        self.searcher.index()
    }

    pub async fn search<C: Collector>(&self, query: &dyn Query, collector: &C) -> Result<C::Fruit> {
        let searcher = self.searcher.clone();
        tokio::task::block_in_place(|| {
            searcher
                .search(query, collector)
                .map_err(|e| Error::Internal(format!("Search failed: {e}")))
        })
    }

    pub async fn doc<D: DocumentDeserialize>(&self, doc_address: DocAddress) -> Result<D> {
        let searcher = self.searcher.clone();
        tokio::task::block_in_place(|| {
            let doc: D = searcher
                .doc(doc_address)
                .map_err(|e| Error::Internal(format!("Failed to get doc: {e}")))?;
            Ok(doc)
        })
    }

    pub async fn num_docs(&self) -> Result<u64> {
        tokio::task::block_in_place(|| Ok(self.searcher.num_docs()))
    }

    pub async fn doc_freq(&self, term: &Term) -> Result<u64> {
        tokio::task::block_in_place(|| Ok(self.searcher.doc_freq(term)?))
    }

    pub async fn space_usage(&self) -> Result<SearcherSpaceUsage> {
        tokio::task::block_in_place(|| Ok(self.searcher.space_usage()?))
    }
}
