use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::services::search::import::ContentIngestionManager;
use crate::services::search::index::IndexManager;
use crate::services::search::schema::unified::{UnifiedIndex, UnifiedSchema};
use crate::services::search::searcher::content::ContentSearcher;
use crate::{error::Result, ServerStateInner};

mod directory;
mod import;
mod index;
mod schema;
mod searcher;
mod service;
mod tokenizer;
mod util;

pub struct ServiceSearch {
    state: Arc<ServerStateInner>,
    index_manager: IndexManager,

    /// searcher for messages, channels, rooms, and other generic content
    content_searcher: OnceCell<Arc<ContentSearcher>>,
    // NOTE: maybe i should just have one *massive* index for EVERYTHING?
    // /// index for room (and server) analytics
    // room_analytics: ActorRef<IndexActor>,

    // /// index for document history
    // document_history: ActorRef<IndexActor>,
}

impl ServiceSearch {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        let index_manager = IndexManager::new(Arc::clone(&state));

        Self {
            state,
            index_manager,
            content_searcher: OnceCell::new(),
            // room_analytics,
            // document_history,
        }
    }

    async fn get_content_searcher(&self) -> Result<Arc<ContentSearcher>> {
        let server_state = Arc::clone(&self.state);

        self.content_searcher
            .get_or_try_init(|| async {
                // open or create the index
                let (writer, reader) = self.index_manager.open(UnifiedIndex::default()).await?;

                // begin (re)indexing channels
                ContentIngestionManager::start(server_state, writer).await?;

                Ok(Arc::new(ContentSearcher::new(
                    reader,
                    UnifiedSchema::default(),
                )))
            })
            .await
            .cloned()
    }
}
