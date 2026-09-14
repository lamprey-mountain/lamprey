use common::{
    v1::types::{
        ChannelCreate, ChannelPatch, MessageCreate, MessagePatch, RoomMember, User,
        automod::{
            AutomodMediaLocation, AutomodRuleTestRequest, AutomodTarget, AutomodTextLocation,
        },
        message::MessageAttachmentCreateType,
    },
    v2::types::{
        MediaId,
        media::{Media, MediaCreate, MediaCreateSource, MediaPatch},
    },
};

/// this is a top level thing that can be scanned by the automod service
pub trait ScannableTarget: Scannable {
    /// Returns the target type of the scannable item.
    fn target(&self) -> AutomodTarget;
}

/// this thing can be scanned
pub trait Scannable {
    /// Visits every piece of scannable text or media within the item.
    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S);
}

/// A visitor trait for handling scanned item fields.
trait Scanner<'a> {
    fn visit_scannable<S: Scannable>(&mut self, scannable: &'a S);

    /// Handles a piece of text component.
    fn visit_text(&mut self, text: &'a str, location: AutomodTextLocation);

    /// Handles a media component.
    fn visit_media(&mut self, media: MediaId, location: AutomodMediaLocation);
}

/// utility to collect all scannable text from a Scannable
// TODO: make all these fields private
pub struct ScannableSet<'a> {
    pub(super) target: AutomodTarget,
    pub(super) text: Vec<(&'a str, AutomodTextLocation)>,
    pub(super) media: Vec<(MediaId, AutomodMediaLocation)>,
}

impl ScannableSet<'_> {
    pub fn new(target: AutomodTarget) -> Self {
        Self {
            target,
            text: vec![],
            media: vec![],
        }
    }
}

impl<'a> Scanner<'a> for ScannableSet<'a> {
    fn visit_text(&mut self, text: &'a str, location: AutomodTextLocation) {
        self.text.push((text, location));
    }

    fn visit_media(&mut self, media: MediaId, location: AutomodMediaLocation) {
        self.media.push((media, location));
    }

    fn visit_scannable<S: Scannable>(&mut self, scannable: &'a S) {
        scannable.scan(self);
    }
}

impl ScannableTarget for MessageCreate {
    fn target(&self) -> AutomodTarget {
        AutomodTarget::Content
    }
}

impl ScannableTarget for MessagePatch {
    fn target(&self) -> AutomodTarget {
        AutomodTarget::Content
    }
}

impl ScannableTarget for ChannelCreate {
    fn target(&self) -> AutomodTarget {
        AutomodTarget::Content
    }
}

impl ScannableTarget for ChannelPatch {
    fn target(&self) -> AutomodTarget {
        AutomodTarget::Content
    }
}

impl<'a> ScannableTarget for (&'a RoomMember, &'a User) {
    fn target(&self) -> AutomodTarget {
        AutomodTarget::Member
    }
}

impl ScannableTarget for AutomodRuleTestRequest {
    fn target(&self) -> AutomodTarget {
        self.target
    }
}

impl Scannable for MessageCreate {
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
    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S) {
        visitor.visit_text(&self.name, AutomodTextLocation::ThreadTitle);

        if let Some(t) = &self.description {
            visitor.visit_text(t, AutomodTextLocation::ThreadTopic);
        }
    }
}

impl Scannable for ChannelPatch {
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
    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S) {
        visitor.visit_text(&self.text, AutomodTextLocation::Test);
    }
}

impl Scannable for MediaCreate {
    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S) {
        if let MediaCreateSource::Download {
            filename: Some(t),
            size: _,
            source_url: _,
        }
        | MediaCreateSource::Upload {
            filename: t,
            size: _,
        } = &self.source
        {
            visitor.visit_text(t, AutomodTextLocation::MediaFilename);
        }

        if let Some(t) = &self.alt {
            visitor.visit_text(t, AutomodTextLocation::MediaAlt);
        }
    }
}

impl Scannable for MediaPatch {
    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S) {
        if let Some(Some(t)) = &self.alt {
            visitor.visit_text(t, AutomodTextLocation::MediaFilename);
        }

        if let Some(t) = &self.filename {
            visitor.visit_text(t, AutomodTextLocation::MediaAlt);
        }
    }
}

impl Scannable for Media {
    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S) {
        visitor.visit_text(&self.filename, AutomodTextLocation::MediaFilename);

        if let Some(t) = &self.alt {
            visitor.visit_text(t, AutomodTextLocation::MediaAlt);
        }

        // TODO: deny quarantined media
        // TODO: handle media.scans
        // if let Some(t) = &self.quarantine {
        //     visitor.visit_text(t, todo!());
        // }

        // visitor.visit_media(media, location);
        // self.scans;
    }
}
