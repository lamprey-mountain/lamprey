//! etl system for loading content into tantivy

use std::sync::Arc;

use crate::{
    Result, ServerStateInner,
    services::search::{
        import::{backfill::BackfillEtl, live::LiveEtl},
        index::AsyncIndexHandle,
    },
};

mod backfill;
mod live;

/// importer for the content index
pub struct IndexEtl {
    s: Arc<ServerStateInner>,
    index: AsyncIndexHandle,
}

impl IndexEtl {
    pub async fn start(s: Arc<ServerStateInner>, index: AsyncIndexHandle) -> Result<()> {
        let live = LiveEtl::new(Arc::clone(&s), index.clone());
        tokio::spawn(live.spawn());

        let backfill = BackfillEtl::new(Arc::clone(&s), index.clone());
        tokio::spawn(backfill.spawn());

        Ok(())
    }
}
