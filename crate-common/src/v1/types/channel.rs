#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::error::{ApiError, ErrorCode};
use crate::v1::types::preferences::PreferencesChannel;
use crate::v1::types::tag::Tag;
use crate::v1::types::util::Time;
use crate::v1::types::{util::Diff, ChannelVerId, PermissionOverwrite};
use crate::v1::types::{
    MediaId, MessageCreate, MessageId, MessageVerId, RoleId, TagId, ThreadMember, User,
};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use super::calendar::{Calendar, CalendarPatch};
use super::document::{Document, DocumentPatch, Wiki, WikiPatch};
use super::{ChannelId, RoomId, UserId};

/// A channel
// TODO(#878): minimal data for channels
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Channel {
    pub id: ChannelId,
    pub room_id: Option<RoomId>,
    pub creator_id: UserId,

    /// owner of the group dm
    pub owner_id: Option<UserId>,

    /// only updates when the channel itself is updated, not the stuff in the channel
    pub version_id: ChannelVerId,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    // TODO: maybe rename this to topic? since most other platforms call it topic.
    // i guess i could also have topic and description be separate (short vs long)
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    pub description: Option<String>,

    /// url that this info channel should link to
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(url, length(min = 1, max = 2048)))]
    pub url: Option<String>,

    /// type specific data for this channel
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: ChannelType,

    /// number of people in this room
    pub member_count: u64,

    /// number of people who are online in this room
    pub online_count: u64,

    /// number of tags in this Forum, Forum2, or Ticket channel
    #[cfg_attr(feature = "serde", serde(default))]
    pub tag_count: u64,

    /// tags that are applied to this thread
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub tags: Option<Vec<TagId>>,

    /// the tags that are available in this forum. exists on Forum channels only.
    // NOTE: if i want to have unlimited tags, i'd have to remove this
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub tags_available: Option<Vec<Tag>>,

    // TODO: rename to removed_at
    pub deleted_at: Option<Time>,
    pub archived_at: Option<Time>,

    /// whether this channel is locked and has restricted permissions
    ///
    /// a locked channel can only be interacted with (sending messages,
    /// (un)archiving, etc) by anyone who has any of
    ///
    /// - a role in allowed_roles
    /// - the `ChannelManage` permission
    /// - the `ThreadLock` or `ThreadManage` permission IF this channel is a thread
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub locked: Option<Locked>,

    /// the channel this channel is in, if any
    pub parent_id: Option<ChannelId>,

    /// the position of this channel in the navbar
    ///
    /// - lower numbers come first (0 is the first channel)
    /// - channels with the same position are tiebroken by id
    /// - channels without a position come last, ordered by newest first
    pub position: Option<u16>,

    /// permission overwrites for this channel
    pub permission_overwrites: Vec<PermissionOverwrite>,

    /// not safe for work
    #[cfg_attr(feature = "serde", serde(default))]
    pub nsfw: bool,

    pub last_version_id: Option<MessageVerId>,
    pub last_message_id: Option<MessageId>,
    pub message_count: Option<u64>,
    pub root_message_count: Option<u64>,

    /// bitrate, for voice channels. defaults to 65535 (64Kibps).
    #[cfg_attr(feature = "validator", validate(range(min = 8192)))]
    pub bitrate: Option<u64>,

    /// maximum number of users who can be in this voice channel
    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
    pub user_limit: Option<u64>,

    // NOTE: consider removing this, its not really being used?
    // the idea was that ignored/muted users could skip incrementing is_unread, but that would require each event to be separately filtered per user
    // individual filtering has some performance implications that i dont know if i want to take on
    pub is_unread: Option<bool>,
    pub last_read_id: Option<MessageVerId>,
    pub mention_count: Option<u64>,
    pub preferences: Option<PreferencesChannel>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub document: Option<Document>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub wiki: Option<Wiki>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub calendar: Option<Calendar>,

    /// for dm and gdm channels, this is who the dm is with
    #[cfg_attr(feature = "serde", serde(default))]
    pub recipients: Vec<User>,

    /// for gdm channels, a custom icon
    pub icon: Option<MediaId>,

    /// whether users without ThreadManage can add other members to this thread
    #[cfg_attr(feature = "serde", serde(default))]
    pub invitable: bool,

    /// The user's thread member object, if the channel is a thread and the user is a member.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub thread_member: Option<Box<ThreadMember>>,

    /// when to automatically archive this thread due to inactivity, in seconds
    pub auto_archive_duration: Option<u64>,

    /// the default auto archive duration in seconds to copy to threads created in this channel
    pub default_auto_archive_duration: Option<u64>,

    /// minimum delay in seconds between creating new threads
    ///
    /// can only be set on channels with has_threads. must have ChannelManage permission to change.
    pub slowmode_thread: Option<u64>,

    /// minimum delay in seconds between creating new messages
    ///
    /// can only be set on channels with text. must have ChannelManage permission to change, or ThreadManage if this is a thread.
    pub slowmode_message: Option<u64>,

    /// default slowmode_message for new threads
    ///
    /// this value is copied, changing this wont change old threads. can only be set on channels with has_threads. must have ChannelManage permission to change.
    pub default_slowmode_message: Option<u64>,

    /// when the current user can create a new thread
    pub slowmode_thread_expire_at: Option<Time>,

    /// when the current user can create a new message
    pub slowmode_message_expire_at: Option<Time>,
}

#[cfg(feature = "serde")]
impl Serialize for Channel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct ChannelView<'a> {
            id: ChannelId,
            #[serde(skip_serializing_if = "Option::is_none")]
            room_id: Option<RoomId>,
            creator_id: UserId,
            #[serde(skip_serializing_if = "Option::is_none")]
            owner_id: Option<UserId>,
            version_id: ChannelVerId,
            name: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            url: Option<&'a str>,
            #[serde(rename = "type")]
            ty: ChannelType,
            member_count: u64,
            online_count: u64,

            #[serde(skip_serializing_if = "Option::is_none")]
            tag_count: Option<u64>,

            #[serde(skip_serializing_if = "Option::is_none")]
            tags: Option<&'a [TagId]>,

            #[serde(skip_serializing_if = "Option::is_none")]
            tags_available: Option<&'a [Tag]>,

            #[serde(skip_serializing_if = "Option::is_none")]
            deleted_at: Option<Time>,
            #[serde(skip_serializing_if = "Option::is_none")]
            archived_at: Option<Time>,

            #[serde(skip_serializing_if = "Option::is_none")]
            locked: Option<&'a Locked>,

            #[serde(skip_serializing_if = "Option::is_none")]
            parent_id: Option<ChannelId>,

            #[serde(skip_serializing_if = "Option::is_none")]
            position: Option<u16>,

            #[serde(skip_serializing_if = "Option::is_none")]
            permission_overwrites: Option<&'a [PermissionOverwrite]>,

            #[serde(skip_serializing_if = "is_false")]
            nsfw: bool,

            #[serde(skip_serializing_if = "Option::is_none")]
            last_version_id: Option<MessageVerId>,
            #[serde(skip_serializing_if = "Option::is_none")]
            last_message_id: Option<MessageId>,
            #[serde(skip_serializing_if = "Option::is_none")]
            message_count: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            root_message_count: Option<u64>,

            #[serde(skip_serializing_if = "Option::is_none")]
            bitrate: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            user_limit: Option<u64>,

            #[serde(skip_serializing_if = "Option::is_none")]
            is_unread: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            last_read_id: Option<MessageVerId>,
            #[serde(skip_serializing_if = "Option::is_none")]
            mention_count: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            preferences: Option<&'a PreferencesChannel>,

            #[serde(skip_serializing_if = "Option::is_none")]
            document: Option<&'a Document>,
            #[serde(skip_serializing_if = "Option::is_none")]
            wiki: Option<&'a Wiki>,
            #[serde(skip_serializing_if = "Option::is_none")]
            calendar: Option<&'a Calendar>,

            #[serde(skip_serializing_if = "Option::is_none")]
            recipients: Option<&'a [User]>,

            #[serde(skip_serializing_if = "Option::is_none")]
            icon: Option<Option<MediaId>>,

            #[serde(skip_serializing_if = "is_false")]
            invitable: bool,

            #[serde(skip_serializing_if = "Option::is_none")]
            thread_member: Option<&'a ThreadMember>,

            #[serde(skip_serializing_if = "Option::is_none")]
            auto_archive_duration: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            default_auto_archive_duration: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            slowmode_thread: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            slowmode_message: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            default_slowmode_message: Option<u64>,

            #[serde(skip_serializing_if = "Option::is_none")]
            slowmode_thread_expire_at: Option<Time>,
            #[serde(skip_serializing_if = "Option::is_none")]
            slowmode_message_expire_at: Option<Time>,
        }

        fn is_false(b: &bool) -> bool {
            !*b
        }

        let view = ChannelView {
            id: self.id,
            room_id: self.room_id,
            creator_id: self.creator_id,
            owner_id: self.owner_id,
            version_id: self.version_id.clone(),
            name: &self.name,
            description: self.description.as_deref(),
            url: if self.ty.has_url() {
                self.url.as_deref()
            } else {
                None
            },
            ty: self.ty,
            member_count: self.member_count,
            online_count: self.online_count,
            tag_count: if self.ty.has_tags() {
                Some(self.tag_count)
            } else {
                None
            },
            tags: if self.ty.is_taggable() {
                Some(self.tags.as_deref().unwrap_or(&[]))
            } else {
                None
            },
            tags_available: if self.ty.has_tags() {
                self.tags_available.as_deref()
            } else {
                None
            },
            deleted_at: self.deleted_at,
            archived_at: self.archived_at,
            locked: self.locked.as_ref(),
            parent_id: self.parent_id,
            position: self.position,
            permission_overwrites: if self.ty.has_permission_overwrites() {
                Some(&self.permission_overwrites)
            } else {
                None
            },
            nsfw: self.nsfw,
            last_version_id: self.last_version_id,
            last_message_id: self.last_message_id,
            message_count: if self.has_text() {
                self.message_count
            } else {
                None
            },
            root_message_count: if self.has_text() {
                self.root_message_count
            } else {
                None
            },
            bitrate: if self.has_voice() { self.bitrate } else { None },
            user_limit: if self.has_voice() {
                self.user_limit
            } else {
                None
            },
            is_unread: self.is_unread,
            last_read_id: self.last_read_id.clone(),
            mention_count: self.mention_count,
            preferences: self.preferences.as_ref(),
            document: if self.has_document() {
                self.document.as_ref()
            } else {
                None
            },
            wiki: if self.has_wiki() {
                self.wiki.as_ref()
            } else {
                None
            },
            calendar: if self.has_calendar() {
                self.calendar.as_ref()
            } else {
                None
            },
            recipients: if self.has_recipients() {
                Some(&self.recipients)
            } else {
                None
            },
            icon: if self.has_icon() {
                Some(self.icon)
            } else {
                None
            },
            invitable: if self.is_thread() {
                self.invitable
            } else {
                false
            },
            thread_member: if self.is_thread() {
                self.thread_member.as_ref().map(|v| &**v)
            } else {
                None
            },
            auto_archive_duration: if self.is_thread() {
                self.auto_archive_duration
            } else {
                None
            },
            default_auto_archive_duration: if self.has_threads() {
                self.default_auto_archive_duration
            } else {
                None
            },
            slowmode_thread: if self.has_threads() {
                self.slowmode_thread
            } else {
                None
            },
            slowmode_message: if self.has_text() {
                self.slowmode_message
            } else {
                None
            },
            default_slowmode_message: if self.has_threads() {
                self.default_slowmode_message
            } else {
                None
            },
            slowmode_thread_expire_at: if self.has_threads() {
                self.slowmode_thread_expire_at
            } else {
                None
            },
            slowmode_message_expire_at: if self.has_text() {
                self.slowmode_message_expire_at
            } else {
                None
            },
        };

        view.serialize(serializer)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ChannelType {
    /// instant messaging
    #[default]
    Text,

    /// announcement channel (like Text but with announcement-specific features)
    Announcement,

    /// a thread visible to anyone who can see the channel
    ThreadPublic,

    /// a thread that is only visible to thread members
    ThreadPrivate,

    /// a thread used in forums, behaving identically to ThreadPublic
    ThreadForum2,

    /// instant messaging direct message
    Dm,

    /// instant messaging group direct message
    Gdm,

    #[cfg(feature = "feat_thread_type_forums")]
    /// long form chat history
    // NOTE: this will be redone later. Forum will be the type of the parent channel, internal threads will use ThreadFoo channels.
    Forum,

    /// a call
    Voice,

    /// broadcast voice channel for many listeners
    Broadcast,

    /// category for grouping channels together
    Category,

    /// a calendar
    Calendar,

    /// experimental tree style long form chat history (like reddit or hackernews)
    Forum2,

    /// info channel without text/voice/threads
    Info,

    /// A channel for support tickets, where each thread is a private conversation.
    Ticket,

    /// a single document, either in the sidebar (eg. for rules) or in a wiki
    ///
    /// document channels dont count towards the channel or active thread cap (and won't be returned in Ready, when Ready gets more data)
    Document,

    /// a comment thread in a document
    DocumentComment,

    /// a channel that holds documents
    Wiki,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelCreate {
    #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, max_length = 1, min_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    pub description: Option<String>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, format = Uri, max_length = 1, min_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(url, length(min = 1, max = 2048)))]
    pub url: Option<String>,

    pub icon: Option<MediaId>,

    /// The type of this channel
    #[cfg_attr(feature = "serde", serde(default, rename = "type"))]
    pub ty: ChannelType,

    /// tags to apply to this thread (overwrite, not append)
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub tags: Option<Vec<TagId>>,

    /// not safe for work
    #[cfg_attr(feature = "serde", serde(default))]
    pub nsfw: bool,

    /// the recipient(s) for this dm/gdm
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 10)))]
    pub recipients: Option<Vec<UserId>>,

    #[cfg_attr(feature = "validator", validate(range(min = 8192)))]
    pub bitrate: Option<u64>,

    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
    pub user_limit: Option<u64>,

    // required for threads
    pub parent_id: Option<ChannelId>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub permission_overwrites: Vec<PermissionOverwrite>,

    /// whether users without ThreadManage can add other members to this thread
    #[cfg_attr(feature = "serde", serde(default))]
    pub invitable: bool,

    pub auto_archive_duration: Option<u64>,

    pub default_auto_archive_duration: Option<u64>,

    pub slowmode_thread: Option<u64>,

    pub slowmode_message: Option<u64>,

    pub default_slowmode_message: Option<u64>,

    /// the initial message for this thread
    ///
    /// required for Forum2 threads. cannot be used elsewhere.
    pub starter_message: Option<MessageCreate>,
}

// TODO(#874) split out channel create structs
#[cfg(any())]
mod split_channel_types {
    use super::{ChannelId, UserId};
    use crate::v1::types::PermissionOverwrite;
    use crate::v1::types::{ChannelType, MediaId, MessageCreate, TagId};

    /// data needed to create a new channel in a room
    // NOTE: do i allow creating threads with this endpoint?
    pub struct ChannelCreateRoom {
        pub name: String,
        pub description: Option<String>,
        // // room channels can't have icons (yet?)
        // pub icon: Option<MediaId>,
        pub ty: ChannelType,
        pub nsfw: bool,
        pub bitrate: Option<u64>,
        pub user_limit: Option<u64>,
        pub parent_id: Option<ChannelId>,
        pub permission_overwrites: Vec<PermissionOverwrite>,
        pub auto_archive_duration: Option<u64>,
        pub default_auto_archive_duration: Option<u64>,
        pub slowmode_thread: Option<u64>,
        pub slowmode_message: Option<u64>,
        pub default_slowmode_message: Option<u64>,
    }

    // channel create dm
    /// data needed to create a new dm or gdm
    pub struct ChannelCreateDm {
        pub name: String,
        pub description: Option<String>,
        pub icon: Option<MediaId>,
        /// must be Dm or Gdm
        pub ty: ChannelType,
        pub recipients: Option<Vec<UserId>>,
    }

    /// data needed to create a new thread
    // maybe have a separate ThreadCreateForum type too?
    pub struct ThreadCreate {
        pub name: String,
        pub description: Option<String>,

        /// must be ThreadPublic or ThreadPrivate. must be ThreadPublic in forums (remove?)
        pub ty: ChannelType,

        /// tags to apply, only usable in forums
        pub tags: Option<Vec<TagId>>,

        /// the initial message for this thread, required in forums
        pub starter_message: Option<MessageCreate>,

        pub invitable: bool,
        pub auto_archive_duration: Option<u64>,
        pub slowmode_message: Option<u64>,
    }

    /// data needed to create a new thread from a mesage
    pub struct ThreadCreateFromMessage {
        pub name: String,
        pub description: Option<String>,
        /// must be ThreadPublic (remove in this case)
        pub ty: ChannelType,
        pub auto_archive_duration: Option<u64>,
        pub slowmode_message: Option<u64>,
    }

    struct ThreadCreateMaybeUnused {
        // // inherits from parent
        // pub nsfw: bool,
        // // maybe include this as users to include by default?
        // pub recipients: Option<Vec<UserId>>,
        // // exists as route param
        // pub parent_id: Option<ChannelId>,
    }
}

// unlikely to be used
#[cfg(any())]
mod granular_channel_data {
    use crate::v1::types::PermissionOverwrite;

    pub struct Channel {
        voice: Option<ChannelVoice>,
        thread: Option<ChannelThread>,
        threadable: Option<ChannelThreadable>,
        text: Option<ChannelText>,
        room: Option<ChannelRoom>,
    }

    pub struct ChannelVoice {
        pub bitrate: Option<u64>,
        pub user_limit: Option<u64>,
    }

    pub struct ChannelThread {
        pub auto_archive_duration: Option<u64>,
    }

    pub struct ChannelThreadable {
        pub default_auto_archive_duration: Option<u64>,
        pub slowmode_thread: Option<u64>,
        pub default_slowmode_message: Option<u64>,
    }

    pub struct ChannelText {
        pub slowmode_message: Option<u64>,
    }

    // for top level room channels
    pub struct ChannelRoom {
        pub nsfw: bool,
        pub permission_overwrites: Vec<PermissionOverwrite>,
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelPatch {
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 64)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 2048))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub description: Option<Option<String>>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, format = Uri, max_length = 1, min_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(url, length(min = 1, max = 2048)))]
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub url: Option<Option<String>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub icon: Option<Option<MediaId>>,

    /// tags to apply to this thread (overwrite, not append)
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub tags: Option<Vec<TagId>>,

    /// not safe for work
    pub nsfw: Option<bool>,

    #[cfg_attr(feature = "validator", validate(range(min = 8192)))]
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub bitrate: Option<Option<u64>>,

    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub user_limit: Option<Option<u64>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub owner_id: Option<Option<UserId>>,

    pub ty: Option<ChannelType>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub parent_id: Option<Option<ChannelId>>,

    pub archived: Option<bool>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub locked: Option<Option<Locked>>,

    pub invitable: Option<bool>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub auto_archive_duration: Option<Option<u64>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub default_auto_archive_duration: Option<Option<u64>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub slowmode_thread: Option<Option<u64>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub slowmode_message: Option<Option<u64>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub default_slowmode_message: Option<Option<u64>>,

    pub document: Option<DocumentPatch>,
    pub wiki: Option<WikiPatch>,
    pub calendar: Option<CalendarPatch>,
}

/// indicates that a channel is locked
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Locked {
    /// if present, the lock automatically expires and is removed at this time
    pub until: Option<Time>,

    /// if present, users with these roles bypass the lock
    #[cfg_attr(feature = "serde", serde(default))]
    pub allow_roles: Vec<RoleId>,
}

/// reorder some channels
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelReorder {
    /// the channels to reorder
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
    pub channels: Vec<ChannelReorderItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelReorderItem {
    pub id: ChannelId,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub position: Option<Option<u16>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub parent_id: Option<Option<ChannelId>>,
}

impl Diff<Channel> for ChannelPatch {
    fn changes(&self, other: &Channel) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.url.changes(&other.url)
            || self.icon.changes(&other.icon)
            || self.tags.changes(&other.tags)
            || self.nsfw.changes(&other.nsfw)
            || self.bitrate.changes(&other.bitrate)
            || self.user_limit.changes(&other.user_limit)
            || self.owner_id.changes(&other.owner_id)
            || self.ty.changes(&other.ty)
            || self.parent_id.changes(&other.parent_id)
            || self.locked.changes(&other.locked)
            || self.invitable.changes(&other.invitable)
            || self.archived.is_some_and(|a| a != other.is_archived())
            || self
                .auto_archive_duration
                .changes(&other.auto_archive_duration)
            || self
                .default_auto_archive_duration
                .changes(&other.default_auto_archive_duration)
            || self.slowmode_thread.changes(&other.slowmode_thread)
            || self.slowmode_message.changes(&other.slowmode_message)
            || self
                .default_slowmode_message
                .changes(&other.default_slowmode_message)
            || match (&self.document, &other.document) {
                (None, _) => false,
                (Some(_), None) => {
                    // WARN: this should be invalid!
                    false
                }
                (Some(a), Some(b)) => a.changes(b),
            }
            || match (&self.wiki, &other.wiki) {
                (None, _) => false,
                (Some(_), None) => {
                    // WARN: this should be invalid!
                    false
                }
                (Some(a), Some(b)) => a.changes(b),
            }
            || match (&self.calendar, &other.calendar) {
                (None, _) => false,
                (Some(_), None) => {
                    // WARN: this should be invalid!
                    false
                }
                (Some(a), Some(b)) => a.changes(b),
            }
    }
}

impl Channel {
    /// remove private user data
    pub fn strip(self) -> Channel {
        Channel {
            is_unread: None,
            last_read_id: None,
            mention_count: None,
            preferences: None,
            ..self
        }
    }

    #[deprecated]
    pub fn is_locked(&self) -> bool {
        if let Some(locked) = &self.locked {
            if let Some(until) = locked.until {
                if until <= Time::now_utc() {
                    return false;
                }
            }
            return true;
        }
        false
    }

    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    pub fn ensure_unarchived(&self) -> Result<(), ApiError> {
        if self.is_archived() {
            Err(ApiError::from_code(ErrorCode::ChannelArchived))
        } else {
            Ok(())
        }
    }

    pub fn is_removed(&self) -> bool {
        self.deleted_at.is_some()
    }

    pub fn ensure_unremoved(&self) -> Result<(), ApiError> {
        if self.is_removed() {
            Err(ApiError::from_code(ErrorCode::ChannelRemoved))
        } else {
            Ok(())
        }
    }

    pub fn is_thread(&self) -> bool {
        self.ty.is_thread()
    }

    pub fn is_taggable(&self) -> bool {
        self.ty.is_taggable()
    }

    pub fn has_document(&self) -> bool {
        self.ty.has_document()
    }

    pub fn has_wiki(&self) -> bool {
        self.ty.has_wiki()
    }

    pub fn has_recipients(&self) -> bool {
        matches!(self.ty, ChannelType::Dm | ChannelType::Gdm)
    }

    pub fn has_calendar(&self) -> bool {
        self.ty.has_calendar()
    }

    pub fn has_url(&self) -> bool {
        self.ty.has_url()
    }

    pub fn has_icon(&self) -> bool {
        self.ty.has_icon()
    }

    pub fn has_threads(&self) -> bool {
        self.ty.has_threads()
    }

    pub fn has_text(&self) -> bool {
        self.ty.has_text()
    }

    pub fn ensure_has_text(&self) -> Result<(), ApiError> {
        self.ty.ensure_has_text()
    }

    pub fn has_voice(&self) -> bool {
        self.ty.has_voice()
    }

    pub fn ensure_has_voice(&self) -> Result<(), ApiError> {
        self.ty.ensure_has_voice()
    }

    pub fn ensure_has_icon(&self) -> Result<(), ApiError> {
        self.ty.ensure_has_icon()
    }

    pub fn ensure_is_thread(&self) -> Result<(), ApiError> {
        self.ty.ensure_is_thread()
    }

    pub fn ensure_has_calendar(&self) -> Result<(), ApiError> {
        self.ty.ensure_has_calendar()
    }

    pub fn ensure_has_url(&self) -> Result<(), ApiError> {
        self.ty.ensure_has_url()
    }

    pub fn ensure_has_threads(&self) -> Result<(), ApiError> {
        self.ty.ensure_has_threads()
    }
}

impl ChannelType {
    pub fn is_thread(&self) -> bool {
        matches!(
            self,
            ChannelType::ThreadPublic | ChannelType::ThreadPrivate | ChannelType::ThreadForum2
        )
    }

    pub fn ensure_is_thread(&self) -> Result<(), ApiError> {
        if self.is_thread() {
            Ok(())
        } else {
            Err(ApiError::from_code(ErrorCode::InvalidThreadType))
        }
    }

    pub fn has_members(&self) -> bool {
        matches!(
            self,
            ChannelType::ThreadPublic | ChannelType::ThreadPrivate | ChannelType::Gdm
        )
    }

    pub fn has_text(&self) -> bool {
        matches!(
            self,
            ChannelType::ThreadPublic
                | ChannelType::ThreadPrivate
                | ChannelType::ThreadForum2
                | ChannelType::Text
                | ChannelType::Announcement
                | ChannelType::Dm
                | ChannelType::Gdm
                | ChannelType::Voice
                | ChannelType::Broadcast
        )
    }

    pub fn ensure_has_text(&self) -> Result<(), ApiError> {
        if self.has_text() {
            Ok(())
        } else {
            Err(ApiError::from_code(ErrorCode::ChannelDoesntHaveText))
        }
    }

    /// whether public threads can be created inside this channel
    pub fn has_public_threads(&self) -> bool {
        matches!(
            self,
            ChannelType::Text
                | ChannelType::Announcement
                | ChannelType::Dm
                | ChannelType::Gdm
                | ChannelType::Forum
        )
    }

    /// whether this is a forum2 channel
    pub fn has_forum2_threads(&self) -> bool {
        matches!(self, ChannelType::Forum2)
    }

    /// whether private threads can be created inside this channel
    pub fn has_private_threads(&self) -> bool {
        matches!(
            self,
            ChannelType::Text | ChannelType::Dm | ChannelType::Gdm | ChannelType::Ticket
        )
    }

    pub fn has_threads(&self) -> bool {
        self.has_public_threads() || self.has_private_threads() || self.has_forum2_threads()
    }

    pub fn ensure_has_threads(&self) -> Result<(), ApiError> {
        if self.has_threads() {
            Ok(())
        } else {
            Err(ApiError::from_code(ErrorCode::InvalidData))
        }
    }

    pub fn has_voice(&self) -> bool {
        matches!(self, ChannelType::Voice | ChannelType::Broadcast)
    }

    pub fn ensure_has_voice(&self) -> Result<(), ApiError> {
        if self.has_voice() {
            Ok(())
        } else {
            Err(ApiError::from_code(ErrorCode::ChannelDoesntHaveVoice))
        }
    }

    pub fn has_url(&self) -> bool {
        matches!(self, ChannelType::Info)
    }

    pub fn ensure_has_url(&self) -> Result<(), ApiError> {
        if self.has_url() {
            Ok(())
        } else {
            Err(ApiError::from_code(ErrorCode::InvalidData))
        }
    }

    /// for a thread to be taggable, it must be in a channel with has_tags
    pub fn is_taggable(&self) -> bool {
        matches!(
            self,
            ChannelType::ThreadPublic | ChannelType::ThreadPrivate | ChannelType::ThreadForum2
        )
    }

    pub fn has_tags(&self) -> bool {
        matches!(
            self,
            ChannelType::Forum | ChannelType::Forum2 | ChannelType::Ticket
        )
    }

    pub fn has_icon(&self) -> bool {
        matches!(self, ChannelType::Gdm)
    }

    pub fn ensure_has_icon(&self) -> Result<(), ApiError> {
        if self.has_icon() {
            Ok(())
        } else {
            Err(ApiError::from_code(ErrorCode::OnlyGdmCanHaveIcons))
        }
    }

    /// if voice connections in this channel act like calls
    pub fn has_call(&self) -> bool {
        matches!(self, ChannelType::Dm | ChannelType::Gdm)
    }

    pub fn has_document(&self) -> bool {
        matches!(self, ChannelType::Document)
    }

    pub fn has_wiki(&self) -> bool {
        matches!(self, ChannelType::Wiki)
    }

    pub fn has_calendar(&self) -> bool {
        matches!(self, ChannelType::Calendar)
    }

    pub fn ensure_has_calendar(&self) -> Result<(), ApiError> {
        if self.has_calendar() {
            Ok(())
        } else {
            // NOTE: Using a generic error as there isn't a specific one for calendar yet
            Err(ApiError::from_code(ErrorCode::InvalidData))
        }
    }

    pub fn has_permission_overwrites(&self) -> bool {
        !self.is_thread() || !matches!(self, ChannelType::Dm | ChannelType::Gdm)
    }

    pub fn can_change_to(self, other: ChannelType) -> bool {
        match (self, other) {
            (ChannelType::ThreadPublic, ChannelType::ThreadPrivate) => true,
            (ChannelType::ThreadPrivate, ChannelType::ThreadPublic) => true,
            (ChannelType::Text, ChannelType::Announcement) => true,
            (ChannelType::Announcement, ChannelType::Text) => true,
            _ => false,
        }
    }

    /// if the member list subscription logic should restrict the list to thread members (instead of filtering all room members)
    pub fn member_list_uses_thread_members(&self) -> bool {
        matches!(self, ChannelType::Dm | ChannelType::Gdm) || self.is_thread()
    }

    /// whether a channel of this type can be inside a channel of this other type. use None for top level rooms.
    pub fn can_be_in(&self, other: Option<ChannelType>) -> bool {
        match (self, other) {
            // text channels can have public or priate threads
            (ChannelType::ThreadPublic, Some(ChannelType::Text)) => true,
            (ChannelType::ThreadPrivate, Some(ChannelType::Text)) => true,

            // text channels can have public or priate threads
            (ChannelType::ThreadPublic, Some(ChannelType::Announcement)) => true,
            (ChannelType::ThreadPublic, Some(ChannelType::Dm)) => true,
            (ChannelType::ThreadPublic, Some(ChannelType::Gdm)) => true,

            // forum channels only have public threads
            (ChannelType::ThreadPublic, Some(ChannelType::Forum)) => true,

            // forum2 channels only have a special public threads
            (ChannelType::ThreadForum2, Some(ChannelType::Forum2)) => true,

            // ticket channels only have private threads
            (ChannelType::ThreadPrivate, Some(ChannelType::Ticket)) => true,
            (ChannelType::ThreadPrivate, Some(ChannelType::Dm)) => true,
            (ChannelType::ThreadPrivate, Some(ChannelType::Gdm)) => true,

            // rooms and categories can hold non-thread, non-dm channels
            (ChannelType::Text, Some(ChannelType::Category) | None) => true,
            (ChannelType::Announcement, Some(ChannelType::Category) | None) => true,
            (ChannelType::Forum, Some(ChannelType::Category) | None) => true,
            (ChannelType::Voice, Some(ChannelType::Category) | None) => true,
            (ChannelType::Broadcast, Some(ChannelType::Category) | None) => true,
            (ChannelType::Calendar, Some(ChannelType::Category) | None) => true,
            (ChannelType::Forum2, Some(ChannelType::Category) | None) => true,
            (ChannelType::Info, Some(ChannelType::Category) | None) => true,
            (ChannelType::Ticket, Some(ChannelType::Category) | None) => true,
            (ChannelType::Wiki, Some(ChannelType::Category) | None) => true,
            (ChannelType::Document, Some(ChannelType::Category) | None) => true,

            // categories can be in room top level
            (ChannelType::Category, None) => true,

            // documents can be in wikis, document comments can be in documents
            (ChannelType::Document, Some(ChannelType::Wiki)) => true,
            (ChannelType::DocumentComment, Some(ChannelType::Document)) => true,

            // everything else is invalid
            (_, _) => false,
        }
    }
}
