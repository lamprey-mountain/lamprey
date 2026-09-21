use std::collections::HashSet;
use uuid::Uuid;

use crate::prelude::*;
use common::v2::types::{
    ChannelId, MediaId, RoomId, UserId,
    media::{Media, MediaLinkType},
};
use lamprey_backend_data_postgres::{
    Data, MediaLink as DbMediaLink, MediaLinkType as DbMediaLinkType,
};

/// utility to manage media linking and unlinking
// TODO: move this to a shared crate?
#[derive(Debug)]
pub struct MediaLinker<'a> {
    /// user id to enforce media ownership for
    user_id: Option<UserId>,

    /// create these media links
    create: Vec<MediaLinkType>,

    /// delete these media links
    delete: Vec<MediaLinkType>,

    /// all of these media links are compatible with each other
    compatible: HashSet<MediaCompatible>,

    /// all of this media should be linked
    media: Vec<&'a Media>,
}

/// normalized link for compatibility checking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MediaCompatible {
    target_type: DbMediaLinkType,
    target_id: Uuid,
}

impl From<&MediaLinkType> for MediaCompatible {
    fn from(link: &MediaLinkType) -> Self {
        let (target_type, target_id) = match link {
            MediaLinkType::Message { message_id, .. } => (DbMediaLinkType::Message, **message_id),
            MediaLinkType::MessageVersion { version_id, .. } => {
                (DbMediaLinkType::MessageVersion, **version_id)
            }
            MediaLinkType::UserAvatar { user_id } => (DbMediaLinkType::UserAvatar, **user_id),
            MediaLinkType::UserBanner { user_id } => (DbMediaLinkType::UserBanner, **user_id),
            MediaLinkType::ChannelIcon { channel_id } => {
                (DbMediaLinkType::ChannelIcon, **channel_id)
            }
            MediaLinkType::RoomIcon { room_id } => (DbMediaLinkType::RoomIcon, **room_id),
            MediaLinkType::RoomBanner { room_id } => (DbMediaLinkType::RoomBanner, **room_id),
            MediaLinkType::Embed { id } => (DbMediaLinkType::Embed, **id),
            MediaLinkType::CustomEmoji { emoji_id, .. } => {
                (DbMediaLinkType::CustomEmoji, **emoji_id)
            }
            MediaLinkType::Script { script_id, .. } => (DbMediaLinkType::Script, **script_id),
            MediaLinkType::ScriptVersion { version_id, .. } => {
                (DbMediaLinkType::ScriptVersion, **version_id)
            }
            MediaLinkType::Document { document_id, .. } => {
                (DbMediaLinkType::Document, **document_id)
            }
        };
        Self {
            target_type,
            target_id,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MediaLinkerError {
    /// you are not the uploader of this media
    #[error("you are not the uploader of this media")]
    NotUploader(MediaId),

    /// this media appeared multiple times
    #[error("this media appeared multiple times")]
    DuplicateMedia(MediaId),

    /// this media is already linked and the link isn't compatible
    #[error("this media is already linked and the link isn't compatible")]
    // TODO: include media link type
    Conflict(MediaId),

    /// internal server error
    #[error("{0}")]
    Internal(Box<Error>),
}

impl From<Error> for MediaLinkerError {
    fn from(err: Error) -> Self {
        Self::Internal(Box::new(err))
    }
}

impl From<MediaLinkerError> for Error {
    fn from(err: MediaLinkerError) -> Self {
        // TODO: better error
        Error::Internal(err.to_string())
    }
}

impl<'a> MediaLinker<'a> {
    /// create a media linker for a user
    ///
    /// media must be uploaded by this user (ie. user_id field must match this user)
    pub fn new(user_id: UserId) -> Self {
        Self::new_inner(Some(user_id))
    }

    /// create a new empty media linker
    ///
    /// this bypasses the "media can only be linked by its uploader" restriction
    pub fn new_without_user() -> Self {
        Self::new_inner(None)
    }

    #[inline]
    fn new_inner(user_id: Option<UserId>) -> Self {
        Self {
            user_id,
            create: Vec::new(),
            delete: Vec::new(),
            compatible: HashSet::new(),
            media: Vec::new(),
        }
    }

    /// a media link should be created
    ///
    /// these links are marked as compatible
    #[inline]
    pub fn create(&mut self, link: MediaLinkType) -> &mut Self {
        self.create.push(link.clone());
        self.compatible.insert((&link).into());
        self
    }

    /// delete an existing media link
    ///
    /// these links are marked as compatible
    #[inline]
    pub fn delete(&mut self, link: MediaLinkType) -> &mut Self {
        self.delete.push(link.clone());
        self.compatible.insert((&link).into());
        self
    }

    /// mark this as compatible with an existing links
    ///
    /// Normally, an error will occur if any links already exist. Marking a link
    /// as compatible will prevent failure even if that specific link already
    /// exists. This does not require a link to exist.
    #[inline]
    pub fn compatible(&mut self, link: MediaLinkType) -> &mut Self {
        self.compatible.insert((&link).into());
        self
    }

    /// add a piece of media to be linked
    #[inline]
    pub fn media(&mut self, media: &'a Media) -> &mut Self {
        self.media.push(media);
        self
    }

    /// write these links to the database via a transaction
    pub async fn write<Txn: Data + ?Sized>(
        &self,
        txn: &mut Txn,
    ) -> CoreResult<(), MediaLinkerError> {
        // ensure there is no duplicate media
        let mut seen = HashSet::new();
        for media in &self.media {
            if !seen.insert(media.id) {
                return Err(MediaLinkerError::DuplicateMedia(media.id));
            }
        }

        // validate media ownership
        if let Some(user_id) = self.user_id {
            for media in &self.media {
                if media.user_id != Some(user_id) {
                    return Err(MediaLinkerError::NotUploader(media.id));
                }
            }
        }

        // PERF: batch select and update
        for media in &self.media {
            // check if there are any conflicting links
            let links = txn.media_link_select(media.id).await?;
            for link in &links {
                let compatible = MediaCompatible {
                    target_type: link.link_type,
                    target_id: link.target_id,
                };
                if !self.compatible.contains(&compatible) {
                    return Err(MediaLinkerError::Conflict(media.id));
                }
            }

            // create links
            for link in &self.create {
                let comp: MediaCompatible = (&*link).into();
                txn.media_link_insert(media.id, comp.target_id, comp.target_type)
                    .await?;
            }

            // delete links
            for link in &self.delete {
                let comp: MediaCompatible = (&*link).into();
                txn.media_link_delete(comp.target_id, comp.target_type)
                    .await?;
            }

            // update media room_id/channel_id
            let mut room_id = None;
            let mut channel_id = None;

            // TODO: theres probably a cleaner functional way to write this code
            // maybe use the below? make sure to exit early (use first found link).
            // links
            //     .iter()
            //     .map(|l| MediaCompatible {
            //         target_type: l.link_type,
            //         target_id: l.target_id,
            //     })
            //     .chain(self.create.iter().map(|l| l.into()));

            // check existing links
            for link in &links {
                if let Some(rid) = db_link_room_id(link) {
                    room_id = Some(rid);
                }
                if let Some(cid) = db_link_channel_id(link) {
                    channel_id = Some(cid);
                }
            }

            // check new links
            for link in &self.create {
                if let Some(rid) = link_room_id(link) {
                    room_id = Some(rid);
                }
                if let Some(cid) = link_channel_id(link) {
                    channel_id = Some(cid);
                }
            }

            txn.media_update_room_and_channel(media.id, room_id, channel_id)
                .await?;
        }

        Ok(())
    }
}

/// get the channel id for a link
fn link_channel_id(link: &MediaLinkType) -> Option<ChannelId> {
    // FIXME: embeds
    match link {
        MediaLinkType::Message { channel_id, .. }
        | MediaLinkType::MessageVersion { channel_id, .. }
        | MediaLinkType::ChannelIcon { channel_id }
        | MediaLinkType::Script { channel_id, .. }
        | MediaLinkType::ScriptVersion { channel_id, .. }
        | MediaLinkType::Document { channel_id, .. } => Some(*channel_id),
        _ => None,
    }
}

/// get the room id for a link
fn link_room_id(link: &MediaLinkType) -> Option<RoomId> {
    // NOTE: should more media link types (like Message or MessageVersion) have RoomId?
    // for now, i can lookup room id from channel id
    // FIXME: populate room_id field in media
    match link {
        MediaLinkType::RoomIcon { room_id }
        | MediaLinkType::RoomBanner { room_id }
        | MediaLinkType::CustomEmoji { room_id, .. } => Some(*room_id),
        _ => None,
    }
}

/// get the room id for a link from the database
fn db_link_room_id(link: &DbMediaLink) -> Option<RoomId> {
    match link.link_type {
        DbMediaLinkType::RoomIcon | DbMediaLinkType::RoomBanner | DbMediaLinkType::CustomEmoji => {
            Some(link.target_id.into())
        }
        _ => None,
    }
}

/// get the channel id for a link from the database
fn db_link_channel_id(link: &DbMediaLink) -> Option<ChannelId> {
    match link.link_type {
        DbMediaLinkType::Message
        | DbMediaLinkType::MessageVersion
        | DbMediaLinkType::ChannelIcon
        | DbMediaLinkType::Script
        | DbMediaLinkType::ScriptVersion
        | DbMediaLinkType::Document => Some(link.target_id.into()),
        _ => None,
    }
}
