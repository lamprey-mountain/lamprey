//! etl system for loading content into tantivy

use crate::prelude::*;
use crate::{
    Result,
    services::search::{
        import::{backfill::BackfillEtl, live::LiveEtl},
        index::AsyncIndexHandle,
    },
};

mod backfill;
mod live;

/// importer for the content index
pub struct IndexEtl {
    s: Globals,
    index: AsyncIndexHandle,
}

impl IndexEtl {
    pub async fn start(s: Globals, index: AsyncIndexHandle) -> Result<()> {
        let live = LiveEtl::new(s.clone(), index.clone());
        tokio::spawn(live.spawn());

        let backfill = BackfillEtl::new(s.clone(), index.clone());
        tokio::spawn(backfill.spawn());

        Ok(())
    }
}
