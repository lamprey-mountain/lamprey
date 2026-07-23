use tokio::sync::OnceCell;
use tracing::error;

use crate::error::Result;
use crate::prelude::*;
use crate::services::search::import::IndexEtl;
use crate::services::search::index::{AsyncIndex, AsyncIndexHandle};
use crate::services::search::schema::unified::UnifiedIndex;

mod import;
mod index;
mod schema;
mod service;
mod tokenizer;
mod util;

pub use util::visibility::{SearchMediaVisibility, SearchRoomsVisibility};

pub struct ServiceSearch {
    state: Globals,
    index: OnceCell<AsyncIndexHandle>,
}

impl ServiceSearch {
    pub fn new(state: Globals) -> Self {
        Self {
            state,
            index: OnceCell::new(),
        }
    }

    async fn get_index(&self) -> Result<AsyncIndexHandle> {
        let s = self.state.clone();

        self.index
            .get_or_try_init(|| async {
                let def = UnifiedIndex::default();
                let index = AsyncIndex::open(s.clone(), def).await?;
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
