use lamprey_macros::record;

use crate::{
    v1::types::{
        EmbedCreate, MessageId, ParseMentions,
        components::{self, Components},
        metadata::Metadata,
    },
    v2::types::media::MediaReference,
};

#[record]
#[derive(Default)]
pub struct MessageCreate {
    /// the message's content in markdown
    #[schema(min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    pub content: Option<String>,

    /// message attachments
    #[schema(required = false, min_length = 0, max_length = 32)]
    #[validate(length(min = 0, max = 32))]
    #[serde(default)]
    pub attachments: Vec<MessageAttachmentCreate>,

    /// the message this message is replying to
    pub reply_id: Option<MessageId>,

    #[schema(required = false, min_length = 0, max_length = 32)]
    #[validate(length(min = 0, max = 32))]
    #[serde(default)]
    pub embeds: Vec<EmbedCreate>,

    #[serde(default)]
    pub mentions: ParseMentions,

    /// application defined metadata
    pub metadata: Option<Metadata>,

    /// the components for this message
    pub components: Option<Components<components::Create>>,

    /// whether to make this message ephemeral
    #[serde(default)]
    pub ephemeral: bool,
}

/// used in `message_create` and `message_update`
#[record]
#[derive(PartialEq)]
pub struct MessageAttachmentCreate {
    #[serde(flatten)]
    pub ty: MessageAttachmentCreateType,

    /// if this is a spoiler and should be blurred
    #[serde(default)]
    pub spoiler: bool,
}

#[record]
#[derive(PartialEq)]
#[serde(tag = "type")]
pub enum MessageAttachmentCreateType {
    Media {
        #[serde(flatten)]
        media: MediaReference,

        /// Shortcut for setting alt text on the media item
        #[schema(min_length = 1, max_length = 8192)]
        alt: Option<Option<String>>,

        /// Shortcut for setting filename on the media item
        #[schema(required = false, min_length = 1, max_length = 256)]
        filename: Option<String>,
    },

    #[cfg(feature = "feat_message_forwarding")]
    Forward {
        channel_id: ChannelId,
        message_id: MessageId,
    },
}

#[cfg(feature = "validator")]
mod _v {
    use super::*;
    use validator::{Validate, ValidateLength, ValidationError, ValidationErrors};

    impl Validate for MessageAttachmentCreateType {
        fn validate(&self) -> Result<(), ValidationErrors> {
            use serde_json::json;

            let mut errors = ValidationErrors::new();

            match self {
                MessageAttachmentCreateType::Media { alt, filename, .. } => {
                    if let Some(Some(alt_val)) = alt {
                        if !alt_val.validate_length(None, Some(8192), None) {
                            let mut err = ValidationError::new("length");
                            err.add_param("max".into(), &json!(8192));
                            err.add_param("actual".into(), &(alt_val.len() as i64));
                            errors.add("alt", err);
                        }
                    }

                    if let Some(filename_val) = filename {
                        if !filename_val.validate_length(Some(1), Some(256), None) {
                            let mut err = ValidationError::new("length");
                            err.add_param("min".into(), &json!(1));
                            err.add_param("max".into(), &json!(256));
                            err.add_param("actual".into(), &(filename_val.len() as i64));
                            errors.add("filename", err);
                        }
                    }
                }
                #[cfg(feature = "feat_message_forwarding")]
                MessageAttachmentCreateType::Forward { .. } => {}
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
    }
}

impl MessageCreate {
    pub fn is_empty(&self) -> bool {
        self.content.as_ref().is_none_or(|s| s.is_empty())
            && self.attachments.is_empty()
            && self.embeds.is_empty()
            && self.components.as_ref().is_none_or(|c| c.is_empty())
    }

    /// set the content of the message
    pub fn content<S: Into<String>>(mut self, content: S) -> Self {
        self.content = Some(content.into());
        self
    }

    /// set the id of the message to reply to
    pub fn reply_id(mut self, reply_id: MessageId) -> Self {
        self.reply_id = Some(reply_id);
        self
    }

    /// add an embed to the message
    pub fn embed(mut self, embed: EmbedCreate) -> Self {
        self.embeds.push(embed);
        self
    }

    /// set the embeds for the message
    pub fn embeds(mut self, embeds: Vec<EmbedCreate>) -> Self {
        self.embeds = embeds;
        self
    }

    /// add an attachment to the message
    pub fn attachment(mut self, attachment: MessageAttachmentCreate) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// set the attachments for the message
    pub fn attachments(mut self, attachments: Vec<MessageAttachmentCreate>) -> Self {
        self.attachments = attachments;
        self
    }

    /// set the metadata for the message
    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// set the components for the message
    pub fn components(mut self, components: Components<components::Create>) -> Self {
        self.components = Some(components);
        self
    }

    /// set the mentions for the message
    pub fn mentions(mut self, mentions: ParseMentions) -> Self {
        self.mentions = mentions;
        self
    }
}

impl From<String> for MessageCreate {
    fn from(content: String) -> Self {
        Self {
            content: Some(content),
            ..Default::default()
        }
    }
}
