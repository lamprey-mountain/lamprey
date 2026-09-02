// TODO: investigate whether i actually need PartialEq/Eq derives on some types

use lamprey_macros::record;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::components::{self, Components};
use crate::v1::types::e2ee::MlsEpoch;
use crate::v1::types::e2ee::media::EncryptedMedia;
use crate::v1::types::flume::MessageFlume;
use crate::v1::types::metadata::Metadata;
use crate::v1::types::misc::binary::Binary;
use crate::v1::types::reaction::ReactionCounts;
use crate::v1::types::util::{Diff, Time};
use crate::v1::types::{
    ApplicationId, Embed, InteractionId, RoomMember, ThreadMember, User, UserId,
};
use crate::v1::types::{MediaId, RoomId};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use crate::v2::types::media::{Media, MediaReference};

use super::EmbedCreate;
use super::channel::Channel;
use super::{ChannelId, MessageId, MessageVerId};

// TODO: move some of these to parent mod?
pub mod flume;
pub mod mentions;
pub mod metadata;

mod create;
mod message_type;

// TEMP: compat
pub use create::*;
pub use mentions::*;
pub use message_type::*;

/// a message
#[record]
pub struct Message {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub room_id: Option<RoomId>,

    // TODO: rename to something better?
    // this is a bit unwieldy, and incorrect if i fetched an old version
    pub latest_version: MessageVersion,

    /// exists if this message is pinned
    pub pinned: Option<Pinned>,

    #[serde(default)]
    pub reactions: ReactionCounts,

    /// when this message was deleted
    ///
    /// deleted messages can still be viewed by moderators for a period of time, but otherwise cannot be recovered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<Time>,

    /// when this message was removed
    ///
    /// removed messages are hidden for non moderators. they are recoverable by moderators
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed_at: Option<Time>,

    /// when this message was created
    pub created_at: Time,

    /// the id of who sent this message
    pub author_id: UserId,

    /// the associated thread for this message, if one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread: Option<Box<Channel>>,

    /// the associated flume for this message, if one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flume: Option<MessageFlume>,

    /// the associated interaction for this message, if one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction: Option<MessageInteraction>,

    /// whether this message is ephemeral
    ///
    /// ephemeral messages are only visible to the user who created an interaction and aren't stored
    #[serde(default)]
    pub ephemeral: bool,
}

impl Message {
    pub fn reply_id(&self) -> Option<MessageId> {
        // NOTE: should i copy reply id logic from search service?
        // kerosene-services/src/services/search/schema/transform.rs
        match &self.latest_version.message_type {
            MessageType::DefaultMarkdown(m) => m.reply_id,
            // MessageType::MessagePinned(p) => Some(p.pinned_message_id),
            // MessageType::ThreadCreated(m) => m.source_message_id,
            _ => None,
        }
    }
}

/// a message's content at a point in time
// TODO: add error "latest message version cannot be deleted"
#[record]
pub struct MessageVersion {
    pub version_id: MessageVerId,

    /// the id of who this edit. if None, this edit was made by the author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_id: Option<UserId>,

    /// the type and content of this message
    // NOTE: message type generally shouldn't change, but i don't know how to "hoist" the type field to the top level Message struct?
    #[serde(flatten)]
    pub message_type: MessageType,

    /// who this message mentioned
    #[serde(default, skip_serializing_if = "Mentions::is_empty")]
    pub mentions: Mentions,

    /// when this message version was created, use this as edited_at
    pub created_at: Time,

    /// when this message version was deleted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<Time>,
}

impl MessageVersion {
    pub fn strip(mut self) -> Self {
        self.message_type = match self.message_type {
            MessageType::DefaultMarkdown(m) => {
                MessageType::DefaultMarkdown(MessageDefaultMarkdown {
                    content: None,
                    attachments: vec![],
                    metadata: None,
                    reply_id: m.reply_id,
                    embeds: vec![],
                    components: Components::default(),
                })
            }
            m => m,
        };
        self
    }
}

/// information about a pinned message
#[record]
pub struct Pinned {
    /// when this was pinned
    pub time: Time,

    /// the position of this pin. lower numbers come first.
    pub position: u16,
}

/// reorder pinned messages
#[record]
#[derive(PartialEq, Eq)]
pub struct PinsReorder {
    /// the messages to reorder
    #[serde(default)]
    #[validate(length(min = 1, max = 1024))]
    pub messages: Vec<PinsReorderItem>,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct PinsReorderItem {
    pub id: MessageId,

    #[serde(
        default,
        deserialize_with = "some_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub position: Option<Option<u16>>,
}

// TODO: move to mod edit, rename to MessageEdit
// TODO: impl validation, copy MessageCreate logic
#[record]
#[derive(Default)]
pub struct MessagePatch {
    /// the new message content in markdown
    #[schema(min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    #[serde(
        default,
        deserialize_with = "some_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub content: Option<Option<String>>,

    /// message attachments
    #[schema(required = false, min_length = 0, max_length = 32)]
    #[validate(length(min = 0, max = 32))]
    pub attachments: Option<Vec<MessageAttachmentCreate>>,

    /// the message this message is replying to
    #[serde(
        default,
        deserialize_with = "some_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub reply_id: Option<Option<MessageId>>,

    pub embeds: Option<Vec<EmbedCreate>>,

    /// application defined metadata
    ///
    /// passing this will replace metadata
    // TODO: better MetadataPatch struct
    #[serde(
        default,
        deserialize_with = "some_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub metadata: Option<Option<Metadata>>,

    /// the components for this message
    pub components: Option<Components<components::Create>>,
}

/// a basic message, written using markdown
#[record]
pub struct MessageDefaultMarkdown {
    /// the message's content in markdown
    #[schema(min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    // TODO: does this break anything? #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    #[schema(min_length = 1, max_length = 32)]
    #[validate(length(min = 1, max = 32), nested)]
    // TODO: does this break anything? #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<MessageAttachment>,

    /// application defined metadata
    // TODO: hoist to MessageVersion?
    // TODO: don't make an Option, skip if Metadata::is_empty?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// the message this message is replying to
    // TODO: does this break anything? #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_id: Option<MessageId>,

    #[schema(min_length = 1, max_length = 32)]
    #[validate(length(min = 1, max = 32), nested)]
    // TODO: does this break anything? #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,

    /// the components for this message
    #[serde(default, skip_serializing_if = "Components::is_empty")]
    pub components: Components<components::Canonical>,
}

/// an encrypted message
#[record]
pub struct MessageEncrypted {
    pub epoch: MlsEpoch,

    // TODO: find an appropriate size limit for this (how much overhead does mls cause?)
    /// encrypted content of the message
    ///
    /// - decrypts into a MessageDefaultMarkdownEncrypted struct
    /// - encrypted with aes-256-gcm
    pub ciphertext: Binary<65536>,

    // TODO: pub alg: EncryptionAlgorithm,
    // TODO: find an appropriate size limit for this (how much overhead does mls cause?)
    /// the nonce for the ciphertext
    pub nonce: Binary<12>,

    /// the media this message is attached to, for garbage collection
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media_ids: Vec<MediaId>,
}

/// a basic message, written using markdown. for use with e2ee.
#[record]
pub struct MessageDefaultMarkdownEncrypted {
    /// the message's content in markdown
    #[schema(min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    pub content: Option<String>,

    #[schema(min_length = 1, max_length = 32)]
    #[validate(length(min = 1, max = 32), nested)]
    pub attachments: Vec<MessageAttachmentEncrypted>,

    /// application defined metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// the message this message is replying to
    pub reply_id: Option<MessageId>,

    #[schema(min_length = 1, max_length = 32)]
    #[validate(length(min = 1, max = 32), nested)]
    pub embeds: Vec<Embed>,

    /// the components for this message
    #[serde(default, skip_serializing_if = "Components::is_empty")]
    pub components: Components<components::Encrypted>,
}

#[record]
pub struct MessageAttachment {
    #[serde(flatten)]
    pub ty: MessageAttachmentType,

    /// if this is a spoiler and should be blurred
    pub spoiler: bool,
}

#[record]
pub struct MessageAttachmentEncrypted {
    #[serde(flatten)]
    pub ty: MessageAttachmentEncryptedType,

    /// if this is a spoiler and should be blurred
    pub spoiler: bool,
}

#[record]
#[serde(tag = "type")]
pub enum MessageAttachmentEncryptedType {
    /// a piece of media
    Media { media: EncryptedMedia },

    #[cfg(feature = "feat_message_forwarding")]
    /// a forwarded message
    Forward { snapshot: MessageSnapshot },
}

/// a snapshot of a message at a point in time, for forwards
#[record]
pub struct MessageSnapshot {
    pub room_id: Option<RoomId>,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub version_id: MessageVerId,
    pub created_at: Time,

    #[serde(flatten)]
    pub message_type: MessageType,

    /// who this message mentioned
    #[serde(default, skip_serializing_if = "Mentions::is_empty")]
    pub mentions: Mentions,
}

#[record]
#[serde(tag = "type")]
pub enum MessageAttachmentType {
    /// a piece of media
    // or should this be called File? should i differentiate files and media?
    Media { media: Media },

    #[cfg(feature = "feat_message_forwarding")]
    /// a forwarded message
    Forward { snapshot: MessageSnapshot },
    // should i have Embed for explicitly added embeds vs generated embeds?
    // TODO: Geolocation,
    // TODO: Moderation, (automod execution? or should this be a message type?)
    // TODO: reference to a range of a document, for document comments
}

/// the interaction that caused this message to be sent
#[record]
#[derive(Default, PartialEq, Eq)]
pub struct MessageInteraction {
    pub id: InteractionId,
    pub application_id: ApplicationId,

    /// the user who triggered this interaction
    pub user_id: UserId,

    /// the interaction's source message
    ///
    /// if this interaction was triggered by a message component (eg. a button), this is the id of the message the component was on
    pub source_message_id: Option<MessageId>,
}

/// the current status
#[cfg(feature = "feat_interaction_reaction")]
#[record]
#[derive(PartialEq, Eq)]
pub enum InteractionStatus {
    /// This message is still loading, or the action it represents is in progress
    ///
    /// - Will switch to Failed after 5 minutes or 30 seconds without edit
    /// - Can edit without creating message history entry
    /// - Intended for dynamic/streaming responses
    Loading,

    /// The (inter)action this message represents failed
    Failed {
        reason: String,
        // code: InteractionStatusKnownErrorCode,
        can_retry: bool,
    },
}

// enum InteractionStatusKnownErrorCode {
//     Forbidden,
//     Timeout,
//     BadInput,
//     Missing,
//     Conflict,
//     Gone,
//     TooLarge,
//     Cancelled,
//     Ratelimit,
// }

#[record]
#[derive(PartialEq, Eq)]
pub struct MessageMove {
    /// which messages to move
    #[serde(default)]
    #[validate(length(min = 1, max = 128))]
    #[schema(min_length = 1, max_length = 128)]
    pub message_ids: Vec<MessageId>,

    /// the channel to move the messages to
    ///
    /// must be in same room (for now...)
    pub target_channel_id: ChannelId,
}

#[record]
pub struct MessageModerate {
    /// which messages to delete
    #[serde(default)]
    #[validate(length(max = 128))]
    #[schema(min_length = 0, max_length = 128)]
    pub delete: Vec<MessageId>,

    /// which messages to remove
    #[serde(default)]
    #[validate(length(max = 128))]
    #[schema(min_length = 0, max_length = 128)]
    pub remove: Vec<MessageId>,

    /// which messages to restore
    #[serde(default)]
    #[validate(length(max = 128))]
    #[schema(min_length = 0, max_length = 128)]
    pub restore: Vec<MessageId>,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RepliesQuery {
    /// how deeply to fetch replies
    #[cfg_attr(feature = "serde", serde(default = "fn_one"))]
    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 8)))]
    pub depth: u16,

    /// how many replies to fetch per branch
    pub breadth: Option<u16>,

    /// which parent message to fetch replies from, where 0 is the message itself, 1 is its parent, and so on.
    pub context: Option<u16>,
}

/// a response to a replies query
#[record]
pub struct RepliesResponse {
    #[serde(flatten)]
    pub children: RepliesChildren,
    // TODO: return users and room/thread members
}

/// a single message for a replies query
#[record]
pub struct RepliesMessage {
    /// the message itself
    pub message: Message,

    /// the children for this message
    #[serde(flatten)]
    #[schema(no_recursion)]
    pub children: RepliesChildren,
}

/// a list of children for a RepliesItem or the top level
#[record]
pub struct RepliesChildren {
    /// the children for this message
    pub children: Vec<RepliesMessage>,

    /// the total number of replies to this message
    pub count_direct: u64,

    /// the total number of replies to this message, calculated recursively
    pub count_recursive: u64,

    /// the current depth of this message in the tree, or 0 for the top level
    pub depth: u64,

    /// cursor that can be used to fetch more
    pub cursor: Option<String>,

    /// whether there are more messages after the end of the children array
    pub has_more: bool,
}

/// always returns one
fn fn_one() -> u16 {
    1
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct ContextQuery {
    pub to_start: Option<MessageId>,
    pub to_end: Option<MessageId>,
    pub limit: Option<u16>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RatelimitPut {
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub slowmode_thread_expire_at: Option<Option<Time>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub slowmode_message_expire_at: Option<Option<Time>>,
}

#[record]
pub struct ContextResponse {
    pub items: Vec<Message>,

    // TODO: maybe remove this?
    pub total: u64,

    pub has_after: bool,
    pub has_before: bool,
    // TODO: maybe impl these?
    // pub cursor_after: bool,
    // pub cursor_before: bool,
    // TODO: add users, etc...
}

#[record]
#[derive(Default)]
pub struct MessageList {
    pub messages: Vec<Message>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub users: Vec<User>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub room_members: Vec<RoomMember>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thread_members: Vec<ThreadMember>,
    // pub has_after: bool,
    // pub has_before: bool,
}

// TODO: move elsewhere?
// TODO: use for message list routes
// TODO: use for message context route
// TODO: use for thread list routes
#[record]
#[derive(Default)]
#[cfg_attr(feature = "utoipa", derive(IntoParams))]
pub struct WithMembersQuery {
    /// whether to include members in response
    ///
    /// includes `users`, `room_members`, and `thread_members` fields
    #[serde(default, skip_serializing_if = "is_false")]
    pub with_members: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}

// TODO: impl MessageList { fn cursor_before, cursor_after }

impl Diff for MessagePatch {
    type Target = Message;

    fn changes(&self, other: &Message) -> bool {
        match &other.latest_version.message_type {
            MessageType::DefaultMarkdown(m) => {
                // content: Option<Option<String>> vs Option<String>
                if let Some(ref val) = self.content {
                    if val != &m.content {
                        return true;
                    }
                }
                // reply_id: Option<Option<MessageId>> vs Option<MessageId>
                if let Some(ref val) = self.reply_id {
                    if val != &m.reply_id {
                        return true;
                    }
                }
                if self.embeds.is_some() {
                    return true;
                }
                // metadata: Option<Option<MessageMetadata>> vs Option<MessageMetadata>
                if let Some(ref val) = self.metadata {
                    if val != &m.metadata {
                        return true;
                    }
                }
                if self.attachments.as_ref().is_some_and(|a| {
                    a.len() != m.attachments.len()
                        || a.iter().zip(&m.attachments).any(|(a, b)| {
                            if a.spoiler != b.spoiler {
                                return true;
                            }

                            match (&a.ty, &b.ty) {
                                (
                                    MessageAttachmentCreateType::Media {
                                        media,
                                        alt,
                                        filename,
                                    },
                                    MessageAttachmentType::Media {
                                        media: existing_media,
                                    },
                                ) => {
                                    match media {
                                        MediaReference::Media { media_id } => {
                                            if *media_id != existing_media.id {
                                                return true;
                                            }
                                        }
                                        // if we're not referencing the media by id, we're uploading/downloading it
                                        _ => return true,
                                    }

                                    // alt: Option<Option<String>> vs existing_media.alt: Option<String>
                                    (if let Some(alt_val) = alt {
                                        alt_val != &existing_media.alt
                                    } else {
                                        false
                                    }) || filename
                                        .as_ref()
                                        .is_some_and(|f| f != &existing_media.filename)
                                }
                                #[cfg(feature = "feat_message_forwarding")]
                                (
                                    MessageAttachmentCreateType::Forward {
                                        channel_id,
                                        message_id,
                                    },
                                    MessageAttachmentType::Forward { snapshot },
                                ) => {
                                    *channel_id != snapshot.channel_id
                                        || *message_id != snapshot.message_id
                                }
                                #[allow(unreachable_patterns)]
                                _ => true,
                            }
                        })
                }) {
                    return true;
                }
                false
            }
            // this edit is invalid!
            _ => false,
        }
    }

    fn apply(self, mut other: Self::Target) -> Self::Target {
        if let MessageType::DefaultMarkdown(ref mut m) = other.latest_version.message_type {
            if let Some(val) = self.content {
                m.content = val;
            }
            if let Some(val) = self.reply_id {
                m.reply_id = val;
            }
            // TODO: handle embeds apply
            if let Some(val) = self.metadata {
                m.metadata = val;
            }
            // Note: attachments apply requires From<MessageAttachmentCreate> for MessageAttachment
        }
        other
    }
}

impl MessageDefaultMarkdown {
    pub fn is_empty(&self) -> bool {
        self.content.as_ref().is_none_or(|s| s.is_empty())
            && self.attachments.is_empty()
            && self.embeds.is_empty()
    }

    /// remove all content from this message
    pub fn strip(&mut self) {
        self.content = None;
        self.attachments = vec![];
        self.embeds = vec![];
    }
}
