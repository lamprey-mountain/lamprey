use crate::{Error, Result};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v2::types::media::{Media, MediaReference};
use common::v2::types::{MediaId, UserId};
use std::collections::HashSet;

#[derive(Default)]
pub struct MediaRegistry {
    pub known: HashSet<MediaId>,
    pub duplicates: HashSet<MediaId>,
}

#[derive(Default)]
pub struct MediaRegistry2<'a> {
    media: Vec<&'a Media>,
    known: HashSet<MediaId>,
    duplicates: HashSet<MediaId>,
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
                format!(
                    "You've used some media ids multiple times, but media can only be used once. Media ids: {}",
                    dupes.join(", ")
                ),
            )))
        }
    }
}

impl<'a> MediaRegistry2<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, media: &'a Media) {
        if !self.known.insert(media.id) {
            self.duplicates.insert(media.id);
        }
        self.media.push(media);
    }

    pub fn check(&self, user_id: UserId) -> Result<()> {
        self.check_duplicates()?;
        self.check_ownership(user_id)?;
        Ok(())
    }

    fn check_ownership(&self, user_id: UserId) -> Result<()> {
        for m in &self.media {
            if m.user_id != Some(user_id) {
                // TODO: better error
                return Err(Error::MissingPermissions);
            }
        }
        Ok(())
    }

    fn check_duplicates(&self) -> Result<()> {
        if self.duplicates.is_empty() {
            Ok(())
        } else {
            let dupes: Vec<_> = self.duplicates.iter().map(|m| m.to_string()).collect();
            // PERF: this can result in a potentially very long message
            Err(Error::ApiError(ApiError::with_message(
                ErrorCode::DuplicateMediaId,
                format!(
                    "You've used some media ids multiple times, but media can only be used once. Media ids: {}",
                    dupes.join(", ")
                ),
            )))
        }
    }

    pub fn media(&self) -> &[&Media] {
        &self.media
    }
}
