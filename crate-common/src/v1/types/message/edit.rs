use lamprey_macros::record;

use crate::v1::types::components::{self, Components};
use crate::v1::types::metadata::Metadata;
use crate::v1::types::util::{Diff, some_option};
use crate::v1::types::{EmbedCreate, MessageId};

use super::{
    Message, MessageAttachmentCreate, MessageAttachmentCreateType, MessageAttachmentType,
    MessageType,
};

// TODO: rename to MessageEdit
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
                                        crate::v2::types::media::MediaReference::Media {
                                            media_id,
                                        } => {
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
