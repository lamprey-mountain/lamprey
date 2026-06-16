#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use crate::{
    util::Diff,
    v1::types::{
        Locked, PermissionOverwrite, TagId,
        calendar::CalendarPatch,
        document::{DocumentPatch, WikiPatch},
        misc::Time,
    },
    v2::types::{
        ChannelId, MediaId, UserId,
        channel::{
            Channel, ChannelBroadcast, ChannelDm, ChannelInfo, ChannelRoom, ChannelText,
            ChannelThread, ChannelThreaded, ChannelVoice,
        },
    },
};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelUpdate {
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 64)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub name: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 2048))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub description: Option<Option<String>>,

    // NOTE: this should probably be removed, no api allows patching owner_id
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub owner_id: Option<Option<UserId>>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub archived: Option<bool>,

    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub room: Option<ChannelRoomUpdate>,

    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub text: Option<ChannelTextUpdate>,

    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub threaded: Option<ChannelThreadedUpdate>,

    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub thread: Option<ChannelThreadUpdate>,

    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub voice: Option<ChannelVoiceUpdate>,

    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub broadcast: Option<ChannelBroadcastUpdate>,

    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub info: Option<ChannelInfoUpdate>,

    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub dm: Option<ChannelDmUpdate>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub document: Option<DocumentPatch>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub wiki: Option<WikiPatch>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub calendar: Option<CalendarPatch>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Diff)]
#[diff(target = "ChannelRoom")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelRoomUpdate {
    #[cfg_attr(feature = "serde", serde(default))]
    pub position: Option<u16>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub permission_overwrites: Option<Vec<PermissionOverwrite>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub nsfw: Option<bool>,
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub locked: Option<Option<Locked>>,
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub parent_id: Option<Option<ChannelId>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Diff)]
#[diff(target = "ChannelText")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelTextUpdate {
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub slowmode_message: Option<Option<u64>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Diff)]
#[diff(target = "ChannelThreaded")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelThreadedUpdate {
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub slowmode_thread: Option<Option<u64>>,
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub default_auto_archive_duration: Option<Option<u64>>,
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub default_slowmode_message: Option<Option<u64>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelThreadUpdate {
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub auto_archive_duration: Option<Option<u64>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub invitable: Option<bool>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub tags: Option<Vec<TagId>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Diff)]
#[diff(target = "ChannelVoice")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelVoiceUpdate {
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub bitrate: Option<Option<u64>>,
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub user_limit: Option<Option<u64>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Diff)]
#[diff(target = "ChannelBroadcast")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelBroadcastUpdate {
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub broadcaster_id: Option<Option<UserId>>,
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub schedule_id: Option<Option<ChannelId>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Diff)]
#[diff(target = "ChannelInfo")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelInfoUpdate {
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub url: Option<Option<String>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Diff)]
#[diff(target = "ChannelDm")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelDmUpdate {
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub icon: Option<Option<MediaId>>,
}

impl Diff for ChannelUpdate {
    type Target = Channel;

    fn changes(&self, other: &Self::Target) -> bool {
        if self.name.as_ref().is_some_and(|v| v != &other.name) {
            return true;
        }
        if self
            .description
            .as_ref()
            .is_some_and(|v| v != &other.description)
        {
            return true;
        }
        if self.owner_id.as_ref().is_some_and(|v| v != &other.owner_id) {
            return true;
        }
        if self
            .archived
            .is_some_and(|v| v != other.archived_at.is_some())
        {
            return true;
        }

        if let (Some(patch), Some(target)) = (&self.room, &other.room) {
            if patch.changes(target) {
                return true;
            }
        }
        if let (Some(patch), Some(target)) = (&self.text, &other.text) {
            if patch.changes(target) {
                return true;
            }
        }
        if let (Some(patch), Some(target)) = (&self.threaded, &other.threaded) {
            if patch.changes(target) {
                return true;
            }
        }
        if let (Some(patch), Some(target)) = (&self.thread, &other.thread) {
            if patch.changes(target) {
                return true;
            }
        }
        if let (Some(patch), Some(target)) = (&self.voice, &other.voice) {
            if patch.changes(target) {
                return true;
            }
        }
        if let (Some(patch), Some(target)) = (&self.broadcast, &other.broadcast) {
            if patch.changes(target) {
                return true;
            }
        }
        if let (Some(patch), Some(target)) = (&self.info, &other.info) {
            if patch.changes(target) {
                return true;
            }
        }
        if let (Some(patch), Some(target)) = (&self.dm, &other.dm) {
            if patch.changes(target) {
                return true;
            }
        }
        if let (Some(patch), Some(target)) = (&self.document, &other.document) {
            if patch.changes(target) {
                return true;
            }
        }
        if let (Some(patch), Some(target)) = (&self.wiki, &other.wiki) {
            if patch.changes(target) {
                return true;
            }
        }
        if let (Some(patch), Some(target)) = (&self.calendar, &other.calendar) {
            if patch.changes(target) {
                return true;
            }
        }

        false
    }

    fn apply(self, mut other: Self::Target) -> Self::Target {
        if let Some(v) = self.name {
            other.name = v;
        }
        if let Some(v) = self.description {
            other.description = v;
        }
        if let Some(v) = self.owner_id {
            other.owner_id = v;
        }

        if let Some(archived) = self.archived {
            if archived {
                if other.archived_at.is_none() {
                    other.archived_at = Some(Time::now_utc());
                }
            } else {
                other.archived_at = None;
            }
        }

        if let Some(patch) = self.room {
            if let Some(target) = other.room.take() {
                other.room = Some(Box::new(patch.apply(*target)));
            }
        }
        if let Some(patch) = self.text {
            if let Some(target) = other.text.take() {
                other.text = Some(Box::new(patch.apply(*target)));
            }
        }
        if let Some(patch) = self.threaded {
            if let Some(target) = other.threaded.take() {
                other.threaded = Some(Box::new(patch.apply(*target)));
            }
        }
        if let Some(patch) = self.thread {
            if let Some(target) = other.thread.take() {
                other.thread = Some(Box::new(patch.apply(*target)));
            }
        }
        if let Some(patch) = self.voice {
            if let Some(target) = other.voice.take() {
                other.voice = Some(Box::new(patch.apply(*target)));
            }
        }
        if let Some(patch) = self.broadcast {
            if let Some(target) = other.broadcast.take() {
                other.broadcast = Some(Box::new(patch.apply(*target)));
            }
        }
        if let Some(patch) = self.info {
            if let Some(target) = other.info.take() {
                other.info = Some(Box::new(patch.apply(*target)));
            }
        }
        if let Some(patch) = self.dm {
            if let Some(target) = other.dm.take() {
                other.dm = Some(Box::new(patch.apply(*target)));
            }
        }
        if let Some(patch) = self.document {
            if let Some(target) = other.document.take() {
                other.document = Some(Box::new(patch.apply(*target)));
            }
        }
        if let Some(patch) = self.wiki {
            if let Some(target) = other.wiki.take() {
                other.wiki = Some(Box::new(patch.apply(*target)));
            }
        }
        if let Some(patch) = self.calendar {
            if let Some(target) = other.calendar.take() {
                other.calendar = Some(Box::new(patch.apply(*target)));
            }
        }

        other
    }
}

impl Diff for ChannelThreadUpdate {
    type Target = ChannelThread;

    fn changes(&self, other: &Self::Target) -> bool {
        if self
            .auto_archive_duration
            .as_ref()
            .is_some_and(|v| v != &other.auto_archive_duration)
        {
            return true;
        }
        if self.invitable.is_some_and(|v| v != other.invitable) {
            return true;
        }
        if let Some(new_tags) = &self.tags {
            let old_ids: Vec<TagId> = other.tags.iter().flatten().map(|t| t.id).collect();
            if new_tags != &old_ids {
                return true;
            }
        }
        false
    }

    fn apply(self, mut other: Self::Target) -> Self::Target {
        if let Some(v) = self.auto_archive_duration {
            other.auto_archive_duration = v;
        }
        if let Some(v) = self.invitable {
            other.invitable = v;
        }
        // NOTE: tags are skipped in apply because the types dont match!
        other
    }
}
