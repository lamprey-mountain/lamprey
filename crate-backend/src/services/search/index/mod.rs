use std::{path::PathBuf, sync::Arc};

use tantivy::{
    collector::Collector, query::Query, schema::document::DocumentDeserialize,
    space_usage::SearcherSpaceUsage, DocAddress, Index, IndexReader, IndexWriter, Searcher,
    TantivyDocument, Term,
};
use tokio::sync::{mpsc, oneshot};

use crate::{
    services::search::{
        directory::ObjectDirectory, schema::IndexDefinition, tokenizer::DynamicTokenizer,
    },
    Error, Result, ServerStateInner,
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
}

/// a wrapper around tantivy to enable asynchronous searching
pub struct AsyncSearcher {
    searcher: Searcher,
}

enum Message {
    CommitIndex(oneshot::Sender<Result<()>>),
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

impl AsyncIndexHandle {
    async fn send_op(&self, op: Message) -> Result<()> {
        self.chan
            .send(op)
            .await
            .map_err(|_| Error::Internal("AsyncIndex.chan closed".to_string()))
    }

    /// commit the index
    pub async fn commit(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_op(Message::CommitIndex(tx)).await?;
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
            let dir = ObjectDirectory::new(
                s,
                PathBuf::from(format!("tantivy/{name_clone}")),
                PathBuf::from(format!("/tmp/tantivy/{name_clone}")),
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
        };

        tokio::task::spawn_blocking(move || me.spawn());

        Ok(AsyncIndexHandle { chan: tx })
    }

    fn spawn(mut self) {
        // TODO: try to commit every 5 seconds
        while let Some(op) = self.chan.blocking_recv() {
            match op {
                Message::CommitIndex(resp) => _ = resp.send(self.commit()),
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
    }

    fn commit(&mut self) -> Result<()> {
        if self.uncommitted_count > 0 {
            self.writer.commit()?;
            self.reader.reload()?;
            self.uncommitted_count = 0;
        }

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
