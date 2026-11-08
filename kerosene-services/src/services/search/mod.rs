use std::sync::atomic::{AtomicBool, Ordering};

use lamprey_backend_core::types::search::Reindex;
use tokio::sync::OnceCell;
use tracing::{error, warn};

use crate::error::Result;
use crate::prelude::*;
use crate::services::search::import::IndexEtl;
use crate::services::search::index::{AsyncIndex, AsyncIndexHandle};
use crate::services::search::schema::{Doctype, unified::UnifiedIndex};

mod import;
mod index;
mod schema;
mod service;
mod tokenizer;
mod util;

// TEMP: re-export
pub use kerosene_search::visibility::{
    SearchAuditLogVisibility, SearchMediaVisibility, SearchRoomsVisibility,
};

pub struct ServiceSearch {
    state: Globals,
    index: OnceCell<AsyncIndexHandle>,

    /// whether the index was recreated and needs a full reindex
    needs_reindex: AtomicBool,
}

impl ServiceSearch {
    pub fn new(state: Globals) -> Self {
        Self {
            state,
            index: OnceCell::new(),
            needs_reindex: AtomicBool::new(false),
        }
    }

    async fn get_index(&self) -> Result<AsyncIndexHandle> {
        let s = self.state.clone();

        self.index
            .get_or_try_init(|| async {
                let def = UnifiedIndex::default();
                let (index, recreated) = AsyncIndex::open(s.clone(), def).await?;
                if recreated {
                    self.needs_reindex.store(true, Ordering::SeqCst);
                }
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
                return;
            }

            if srv.search.needs_reindex.swap(false, Ordering::SeqCst) {
                warn!("search index was recreated, queueing full reindex");

                let reindex = Reindex {
                    doctypes: vec![
                        Doctype::Message,
                        Doctype::AuditLogEntry,
                        Doctype::Room,
                        Doctype::Channel,
                        Doctype::User,
                        Doctype::Media,
                    ],
                    room_ids: Vec::new(),
                    channel_ids: Vec::new(),
                };

                if let Err(err) = srv.search.reindex(reindex).await {
                    error!("failed to queue reindex after index recreation: {err}");
                    srv.search.needs_reindex.store(true, Ordering::SeqCst);
                }
            }
        });
    }
}
