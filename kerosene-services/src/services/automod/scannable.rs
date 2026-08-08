use common::v1::types::{
    ChannelCreate, ChannelPatch, MessageCreate, MessagePatch, RoomMember, User,
    automod::{AutomodMediaLocation, AutomodRuleTestRequest, AutomodTarget, AutomodTextLocation},
    message::MessageAttachmentCreateType,
};

use crate::services::automod::compiled::Scannable;

use super::compiled::Scanner;

impl Scannable for MessageCreate {
    fn target(&self) -> AutomodTarget {
        AutomodTarget::Content
    }

    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S) {
        if let Some(t) = self.content.as_deref() {
            visitor.visit_text(t, AutomodTextLocation::MessageContent);
        }

        for att in &self.attachments {
            if let MessageAttachmentCreateType::Media { media, .. } = &att.ty {
                if let Some(media_id) = media.media_id() {
                    visitor.visit_media(media_id, AutomodMediaLocation::MessageAttachment);
                }
            }
        }

        for emb in &self.embeds {
            if let Some(t) = &emb.title {
                visitor.visit_text(t, AutomodTextLocation::EmbedTitle);
            }
            if let Some(t) = &emb.description {
                visitor.visit_text(t, AutomodTextLocation::EmbedDescription);
            }
            if let Some(t) = &emb.author_name {
                visitor.visit_text(t, AutomodTextLocation::EmbedAuthorName);
            }
            if let Some(t) = &emb.author_url {
                visitor.visit_text(t.as_str(), AutomodTextLocation::EmbedAuthorUrl);
            }
            if let Some(t) = &emb.url {
                visitor.visit_text(t.as_str(), AutomodTextLocation::EmbedUrl);
            }
        }

        // TODO: scan embed media
        // TODO: scan components
        // same for MessagePatch
    }
}

impl Scannable for MessagePatch {
    fn target(&self) -> AutomodTarget {
        AutomodTarget::Content
    }

    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S) {
        if let Some(Some(s)) = self.content.as_ref() {
            visitor.visit_text(s, AutomodTextLocation::MessageContent);
        }

        if let Some(attachments) = &self.attachments {
            for att in attachments {
                if let MessageAttachmentCreateType::Media { media, .. } = &att.ty {
                    if let Some(media_id) = media.media_id() {
                        visitor.visit_media(media_id, AutomodMediaLocation::MessageAttachment);
                    }
                }
            }
        }

        if let Some(embeds) = &self.embeds {
            for emb in embeds {
                if let Some(t) = &emb.title {
                    visitor.visit_text(t, AutomodTextLocation::EmbedTitle);
                }
                if let Some(t) = &emb.description {
                    visitor.visit_text(t, AutomodTextLocation::EmbedDescription);
                }
                if let Some(t) = &emb.author_name {
                    visitor.visit_text(t, AutomodTextLocation::EmbedAuthorName);
                }
                if let Some(t) = &emb.author_url {
                    visitor.visit_text(t.as_str(), AutomodTextLocation::EmbedAuthorUrl);
                }
                if let Some(t) = &emb.url {
                    visitor.visit_text(t.as_str(), AutomodTextLocation::EmbedUrl);
                }
            }
        }
    }
}

impl Scannable for ChannelCreate {
    fn target(&self) -> AutomodTarget {
        AutomodTarget::Content
    }

    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S) {
        visitor.visit_text(&self.name, AutomodTextLocation::ThreadTitle);

        if let Some(t) = &self.description {
            visitor.visit_text(t, AutomodTextLocation::ThreadTopic);
        }
    }
}

impl Scannable for ChannelPatch {
    fn target(&self) -> AutomodTarget {
        AutomodTarget::Content
    }

    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S) {
        if let Some(name) = &self.name {
            visitor.visit_text(name, AutomodTextLocation::ThreadTitle);
        }
        if let Some(Some(t)) = self.description.as_ref() {
            visitor.visit_text(t, AutomodTextLocation::ThreadTopic);
        }
    }
}

impl<'a> Scannable for (&'a RoomMember, &'a User) {
    fn target(&self) -> AutomodTarget {
        AutomodTarget::Member
    }

    fn scan<'b, S: Scanner<'b>>(&'b self, visitor: &mut S) {
        visitor.visit_text(&self.1.name, AutomodTextLocation::UserName);

        if let Some(t) = &self.1.description {
            visitor.visit_text(t, AutomodTextLocation::UserBio);
        }

        if let Some(t) = &self.0.override_name {
            visitor.visit_text(t, AutomodTextLocation::MemberNickname);
        }

        // NOTE: this may be removed later
        if let Some(t) = &self.0.override_description {
            visitor.visit_text(t, AutomodTextLocation::MemberDescription);
        }
    }
}

impl Scannable for AutomodRuleTestRequest {
    fn target(&self) -> AutomodTarget {
        self.target
    }

    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S) {
        visitor.visit_text(&self.text, AutomodTextLocation::Test);
    }
}
