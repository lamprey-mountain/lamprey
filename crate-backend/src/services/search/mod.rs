use std::sync::Arc;

use tokio::sync::OnceCell;
use tracing::error;

use crate::services::search::import::IndexEtl;
use crate::services::search::index::{AsyncIndex, AsyncIndexHandle};
use crate::services::search::schema::unified::UnifiedIndex;
use crate::{ServerStateInner, error::Result};

mod directory;
mod import;
mod index;
mod schema;
mod service;
mod tokenizer;
mod util;

pub use util::visibility::{SearchMediaVisibility, SearchRoomsVisibility};

pub struct ServiceSearch {
    state: Arc<ServerStateInner>,
    index: OnceCell<AsyncIndexHandle>,
}

impl ServiceSearch {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            index: OnceCell::new(),
        }
    }

    async fn get_index(&self) -> Result<AsyncIndexHandle> {
        let s = Arc::clone(&self.state);

        self.index
            .get_or_try_init(|| async {
                let def = UnifiedIndex::default();
                let index = AsyncIndex::open(Arc::clone(&s), def).await?;
                IndexEtl::start(s, index.clone()).await?;
                Ok(index)
            })
            .await
            .cloned()
    }

    pub fn start_background_tasks(&self) {
        let srv = self.state.services();
        _ = tokio::spawn(async move {
            if let Err(err) = srv.search.get_index().await {
                error!("failed to open index: {err}");
            }
        });
    }
}
