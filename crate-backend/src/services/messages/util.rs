use crate::{Error, Result};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v2::types::media::MediaReference;
use lamprey_backend_data_postgres::MediaId;
use std::collections::HashSet;

#[derive(Default)]
pub struct MediaRegistry {
    pub known: HashSet<MediaId>,
    pub duplicates: HashSet<MediaId>,
}

impl MediaRegistry {
    pub fn insert(&mut self, media_id: MediaId) {
        if !self.known.insert(media_id) {
            self.duplicates.insert(media_id);
        }
    }

    pub fn insert_ref(&mut self, mr: &MediaReference) -> Result<()> {
        let Some(media_id) = mr.media_id() else {
            return Err(Error::Unimplemented);
        };

        self.insert(media_id);
        Ok(())
    }

    // PERF: this could probably be improved
    pub fn extend(&mut self, ids: &[MediaId]) {
        for id in ids {
            self.insert(*id);
        }
    }

    pub fn extend_refs(&mut self, items: &[MediaReference]) {
        for i in items {
            let _ = self.insert_ref(i);
        }
    }

    pub fn check(&self) -> Result<()> {
        if self.duplicates.is_empty() {
            Ok(())
        } else {
            let dupes: Vec<_> = self.duplicates.iter().map(|m| m.to_string()).collect();
            Err(Error::ApiError(ApiError::with_message(
                ErrorCode::DuplicateMediaId,
                format!("You've used some media ids multiple times, but media can only be used once. Media ids: {}", dupes.join(", ")),
            )))
        }
    }
}
