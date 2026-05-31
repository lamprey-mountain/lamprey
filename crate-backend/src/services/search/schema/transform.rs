//! code to transform lamprey data to tantivy documents

use std::collections::BTreeMap;

use common::v1::types::application::Application;
use common::v1::types::calendar::CalendarEvent;
use common::v1::types::document::serialized::Serdoc;
use common::v1::types::document::DocumentChange;
use common::v1::types::emoji::EmojiCustom;
use common::v1::types::message::Message;
use common::v1::types::room_template::RoomTemplate;
use common::v1::types::tag::Tag;
use common::v1::types::util::Time;
use common::v1::types::voice::Call;
use common::v1::types::{
    AuditLogEntry, Channel, ChannelId, ChannelType, MessageAttachmentType, MessageType, Room,
    RoomId, User,
};
use common::v2::types::media::Media;
use lamprey_backend_core::types::analytics::AnalyticsEvent;
use tantivy::schema::OwnedValue;
use tantivy::{DateTime as TantivyDT, TantivyDocument};

use crate::services::search::schema::{Doctype, UnifiedSchema};
use crate::{Error, Result};

impl UnifiedSchema {
    pub fn transform_message(
        &self,
        message: &Message,
        room_id: Option<RoomId>,
        parent_channel_id: Option<ChannelId>,
    ) -> Result<TantivyDocument> {
        let mut doc = TantivyDocument::new();
        doc.add_text(self.id, message.id.to_string());
        doc.add_text(self.doctype, Doctype::Message);
        doc.add_text(self.channel_id, message.channel_id.to_string());

        if let Some(pid) = parent_channel_id {
            doc.add_text(self.parent_channel_id, pid.to_string());
        }

        doc.add_text(self.author_id, message.author_id.to_string());
        doc.add_date(self.created_at, TantivyDT::from_utc(*message.created_at));

        let updated_at = message.latest_version.created_at;
        if updated_at != message.created_at {
            doc.add_date(self.updated_at, TantivyDT::from_utc(*updated_at));
        }

        if let Some(deleted_at) = message.deleted_at {
            doc.add_date(self.deleted_at, TantivyDT::from_utc(*deleted_at));
        }

        if let Some(removed_at) = message.removed_at {
            doc.add_date(self.removed_at, TantivyDT::from_utc(*removed_at));
        }

        if let Some(room_id) = room_id {
            doc.add_text(self.room_id, room_id.to_string());
        }

        doc.add_text(
            self.subtype,
            match &message.latest_version.message_type {
                MessageType::DefaultMarkdown(..) => "DefaultMarkdown",
                MessageType::MessagePinned(..) => "MessagePinned",
                MessageType::MemberAdd(..) => "MemberAdd",
                MessageType::MemberRemove(..) => "MemberRemove",
                MessageType::MemberJoin => "MemberJoin",
                MessageType::Call(..) => "Call",
                MessageType::ChannelRename(..) => "ChannelRename",
                MessageType::ChannelPingback(..) => "ChannelPingback",
                MessageType::ChannelMoved(..) => "ChannelMoved",
                MessageType::ChannelIcon(..) => "ChannelIcon",
                MessageType::ThreadCreated(..) => "ThreadCreated",
                MessageType::AutomodExecution(..) => "AutomodExecution",
            },
        );

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        let mut meta_text: BTreeMap<String, OwnedValue> = BTreeMap::new();

        let reply = match &message.latest_version.message_type {
            MessageType::DefaultMarkdown(m) => m.reply_id,
            MessageType::MessagePinned(p) => Some(p.pinned_message_id),
            MessageType::ThreadCreated(m) => m.source_message_id,
            _ => None,
        };

        doc.add_text(
            self.content,
            message.latest_version.message_type.to_string(),
        );

        if let MessageType::DefaultMarkdown(ref m) = message.latest_version.message_type {
            if !m.attachments.is_empty() {
                meta_fast.insert("has_attachment".to_string(), true.into());

                let has_audio = m.attachments.iter().any(|a| {
                    let MessageAttachmentType::Media { media } = &a.ty;
                    media.content_type.to_string().starts_with("audio/")
                });
                let has_image = m.attachments.iter().any(|a| {
                    let MessageAttachmentType::Media { media } = &a.ty;
                    media.content_type.to_string().starts_with("image/")
                });
                let has_video = m.attachments.iter().any(|a| {
                    let MessageAttachmentType::Media { media } = &a.ty;
                    media.content_type.to_string().starts_with("video/")
                });

                meta_fast.insert("has_audio".to_string(), has_audio.into());
                meta_fast.insert("has_image".to_string(), has_image.into());
                meta_fast.insert("has_video".to_string(), has_video.into());

                for att in &m.attachments {
                    let MessageAttachmentType::Media { media } = &att.ty;
                    let push_val =
                        |map: &mut BTreeMap<String, OwnedValue>, key: &str, val: OwnedValue| {
                            let entry = map
                                .entry(key.to_string())
                                .or_insert_with(|| OwnedValue::Array(Vec::new()));
                            if let OwnedValue::Array(vec) = entry {
                                vec.push(val);
                            }
                        };

                    push_val(&mut meta_fast, "media_size", media.size.into());
                    push_val(
                        &mut meta_fast,
                        "media_content_type",
                        media.content_type.to_string().into(),
                    );
                    push_val(
                        &mut meta_fast,
                        "media_filename",
                        media.filename.as_str().into(),
                    );

                    if let Some(alt) = &media.alt {
                        push_val(&mut meta_text, "media_alt", alt.as_str().into());
                    }

                    let extension = std::path::Path::new(&media.filename)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|ext| ext.to_lowercase());
                    if let Some(e) = extension {
                        push_val(&mut meta_fast, "media_extension", e.as_str().into());
                    }
                }
            } else {
                meta_fast.insert("has_attachment".to_string(), false.into());
                meta_fast.insert("has_audio".to_string(), false.into());
                meta_fast.insert("has_image".to_string(), false.into());
                meta_fast.insert("has_video".to_string(), false.into());
            }

            meta_fast.insert("has_embed".to_string(), (!m.embeds.is_empty()).into());
        };

        meta_fast.insert("has_thread".to_string(), message.thread.is_some().into());
        meta_fast.insert("pinned".to_string(), message.pinned.is_some().into());

        if let Some(reply_id) = reply {
            meta_fast.insert("reply".to_string(), reply_id.to_string().into());
        }

        let mn = &message.latest_version.mentions;
        meta_fast.insert("mentions_everyone".to_string(), mn.everyone.into());

        if !mn.roles.is_empty() {
            let roles: Vec<OwnedValue> = mn.roles.iter().map(|r| r.id.to_string().into()).collect();
            meta_fast.insert("mentions_role".to_string(), OwnedValue::Array(roles));
        }

        if !mn.users.is_empty() {
            let users: Vec<OwnedValue> = mn.users.iter().map(|u| u.id.to_string().into()).collect();
            meta_fast.insert("mentions_user".to_string(), OwnedValue::Array(users));
        }

        let mut has_links = false;
        if let MessageType::DefaultMarkdown(ref m) = message.latest_version.message_type {
            if let Some(ref content) = m.content {
                let mut hostnames = Vec::new();
                for url in crate::services::messages::links::extract_links(content) {
                    if let Some(host) = url.host_str() {
                        let reversed_hostname = host.split('.').rev().collect::<Vec<_>>().join(".");
                        hostnames.push(reversed_hostname.into());
                        has_links = true;
                    }
                }
                if !hostnames.is_empty() {
                    meta_fast.insert("link_hostname".to_string(), OwnedValue::Array(hostnames));
                }
            }
        }
        meta_fast.insert("has_link".to_string(), has_links.into());

        doc.add_object(self.metadata_fast, meta_fast);
        doc.add_object(self.metadata_text, meta_text);

        Ok(doc)
    }

    /// transform a user to a tantivy document
    pub fn transform_user(&self, user: &User) -> Result<TantivyDocument> {
        let mut doc = TantivyDocument::new();
        doc.add_text(self.id, user.id.to_string());
        doc.add_text(self.doctype, Doctype::User);
        doc.add_text(self.name, user.name.clone());

        if let Some(description) = user.description.clone() {
            doc.add_text(self.content, description);
        }

        if let Some(registered_at) = user.registered_at {
            doc.add_date(self.created_at, TantivyDT::from_utc(*registered_at));
        } else {
            let created_at: Time = user.id.try_into().unwrap();
            doc.add_date(self.created_at, TantivyDT::from_utc(*created_at));
        }

        if let Some(deleted_at) = user.deleted_at {
            doc.add_date(self.deleted_at, TantivyDT::from_utc(*deleted_at));
        }

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        meta_fast.insert("bot".to_string(), user.bot.into());
        meta_fast.insert("system".to_string(), user.system.into());
        meta_fast.insert("suspended".to_string(), user.is_suspended().into());

        doc.add_object(self.metadata_fast, meta_fast);

        Ok(doc)
    }

    /// transform a room to a tantivy document
    pub fn transform_room(&self, room: &Room) -> Result<TantivyDocument> {
        let mut doc = TantivyDocument::new();
        doc.add_text(self.id, room.id.to_string());
        doc.add_text(self.doctype, Doctype::Room);
        doc.add_text(self.name, room.name.clone());

        if let Some(description) = &room.description {
            doc.add_text(self.content, description.clone());
        }

        let created_at: Time = room.id.try_into().unwrap();
        doc.add_date(self.created_at, TantivyDT::from_utc(*created_at));

        if let Some(deleted_at) = room.deleted_at {
            doc.add_date(self.deleted_at, TantivyDT::from_utc(*deleted_at));
        }

        if let Some(archived_at) = room.archived_at {
            doc.add_date(self.archived_at, TantivyDT::from_utc(*archived_at));
        }

        if let Some(owner_id) = room.owner_id {
            doc.add_text(self.author_id, owner_id.to_string());
        }

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        meta_fast.insert("public".to_string(), room.public.into());
        meta_fast.insert("member_count".to_string(), room.member_count.into());
        meta_fast.insert("quarantined".to_string(), room.quarantined.into());

        doc.add_object(self.metadata_fast, meta_fast);

        Ok(doc)
    }

    /// transform a channel to a tantivy document
    pub fn transform_channel(&self, channel: &Channel) -> Result<TantivyDocument> {
        let mut doc = TantivyDocument::new();
        doc.add_text(self.id, channel.id.to_string());
        doc.add_text(self.doctype, Doctype::Channel);
        let last_activity: Time = channel
            .archived_at
            .or_else(|| channel.last_version_id.and_then(|id| id.try_into().ok()))
            .or_else(|| channel.id.try_into().ok())
            .unwrap();
        doc.add_date(self.updated_at, TantivyDT::from_utc(*last_activity));
        doc.add_text(self.name, channel.name.clone());

        if let Some(description) = &channel.description {
            doc.add_text(self.content, description.clone());
        }

        if let Some(room_id) = channel.room_id {
            doc.add_text(self.room_id, room_id.to_string());
        }

        if let Some(parent_id) = channel.parent_id {
            doc.add_text(self.channel_id, parent_id.to_string());
        }

        if let Some(owner_id) = channel.owner_id.map(|i| i.to_string()) {
            doc.add_text(self.author_id, owner_id);
        }

        if let Some(tags) = &channel.tags {
            for tag_id in tags {
                doc.add_text(self.tag_id, tag_id.to_string());
            }
        }

        let created_at: Time = channel.id.try_into().unwrap();
        doc.add_date(self.created_at, TantivyDT::from_utc(*created_at));

        if let Some(deleted_at) = channel.deleted_at {
            doc.add_date(self.deleted_at, TantivyDT::from_utc(*deleted_at));
        }

        if let Some(archived_at) = channel.archived_at {
            doc.add_date(self.archived_at, TantivyDT::from_utc(*archived_at));
        }

        doc.add_text(
            self.subtype,
            match channel.ty {
                ChannelType::Text => "Text",
                ChannelType::Announcement => "Announcement",
                ChannelType::ThreadPublic => "ThreadPublic",
                ChannelType::ThreadPrivate => "ThreadPrivate",
                ChannelType::ThreadForum2 => "ThreadForum2",
                ChannelType::Dm => "Dm",
                ChannelType::Gdm => "Gdm",
                ChannelType::Forum => "Forum",
                ChannelType::Voice => "Voice",
                ChannelType::Broadcast => "Broadcast",
                ChannelType::Category => "Category",
                ChannelType::Calendar => "Calendar",
                ChannelType::Forum2 => "Forum2",
                ChannelType::Info => "Info",
                ChannelType::Ticket => "Ticket",
                ChannelType::Document => "Document",
                ChannelType::DocumentComment => "DocumentComment",
                ChannelType::Wiki => "Wiki",
                ChannelType::Scripts => "Scripts",
            },
        );

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        meta_fast.insert("nsfw".to_string(), channel.nsfw.into());

        if let Some(bitrate) = channel.bitrate {
            meta_fast.insert("bitrate".to_string(), bitrate.into());
        }

        if let Some(user_limit) = channel.user_limit {
            meta_fast.insert("user_limit".to_string(), user_limit.into());
        }

        doc.add_object(self.metadata_fast, meta_fast);

        Ok(doc)
    }

    /// transform media to a tantivy document
    pub fn transform_media(&self, media: &Media) -> Result<TantivyDocument> {
        let user_id = media
            .user_id
            .ok_or_else(|| Error::Internal("Media missing user_id".to_string()))?;

        let mut doc = TantivyDocument::new();

        doc.add_text(self.id, media.id.to_string());
        doc.add_text(self.doctype, Doctype::Media);

        let created_at: Time = media
            .id
            .try_into()
            .map_err(|_| Error::Internal("Invalid media id format".to_string()))?;
        doc.add_date(self.created_at, TantivyDT::from_utc(*created_at));
        doc.add_text(self.author_id, user_id.to_string());

        if let Some(r) = media.room_id {
            doc.add_text(self.room_id, r.to_string());
        }

        if let Some(r) = media.channel_id {
            doc.add_text(self.channel_id, r.to_string());
        }

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        let mut meta_text: BTreeMap<String, OwnedValue> = BTreeMap::new();

        meta_fast.insert("media_size".to_string(), media.size.into());
        meta_fast.insert(
            "media_content_type".to_string(),
            media.content_type.to_string().into(),
        );
        meta_fast.insert("media_filename".to_string(), media.filename.clone().into());

        let extension = std::path::Path::new(&media.filename)
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| ext.to_lowercase());
        if let Some(e) = extension {
            meta_fast.insert("media_extension".to_string(), e.into());
        }

        if let Some(alt) = &media.alt {
            meta_text.insert("media_alt".to_string(), alt.clone().into());
        }

        meta_fast.insert("quarantined".to_string(), media.quarantine.is_some().into());

        doc.add_object(self.metadata_fast, meta_fast);
        doc.add_object(self.metadata_text, meta_text);

        Ok(doc)
    }

    pub fn transform_audit_log_entry(&self, ent: &AuditLogEntry) -> Result<TantivyDocument> {
        let mut doc = TantivyDocument::new();
        doc.add_text(self.id, ent.id.to_string());
        doc.add_text(self.doctype, Doctype::AuditLogEntry);
        doc.add_text(self.room_id, ent.room_id.to_string());
        doc.add_text(self.author_id, ent.user_id.to_string());
        doc.add_date(self.created_at, TantivyDT::from_utc(*ent.started_at));

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        meta_fast.insert("status".to_string(), format!("{:?}", ent.status).into());
        meta_fast.insert("audit_event".to_string(), format!("{:?}", ent.ty).into());

        if let Some(app_id) = ent.application_id {
            meta_fast.insert("application_id".to_string(), app_id.to_string().into());
        }

        doc.add_object(self.metadata_fast, meta_fast);

        Ok(doc)
    }

    /// transform an analytics event to a tantivy document
    pub fn transform_analytics_event(&self, event: &AnalyticsEvent) -> Result<TantivyDocument> {
        todo!()
    }

    /// transform a serialized lamprey document to a tantivy document
    pub fn transform_document(
        &self,
        document: &Channel,
        serialized: &Serdoc,
    ) -> Result<TantivyDocument> {
        todo!()
    }

    /// transform a single lamprey document change to a tantivy document
    pub fn transform_document_change(
        &self,
        change: &DocumentChange,
        document: &Channel,
    ) -> Result<TantivyDocument> {
        todo!()
    }

    /// transform a call into a tantivy document
    ///
    /// for public broadcasts
    pub fn transform_call(&self, call: &Call, channel: &Channel) -> Result<TantivyDocument> {
        todo!()
    }

    // TODO: add these
    pub fn transform_custom_emoji(&self, emoji: EmojiCustom) -> Result<TantivyDocument> {
        todo!()
    }

    pub fn transform_forum_tag(&self, tag: &Tag, channel: &Channel) -> Result<TantivyDocument> {
        todo!()
    }

    pub fn transform_application(&self, app: &Application) -> Result<TantivyDocument> {
        todo!()
    }

    pub fn transform_room_template(&self, template: &RoomTemplate) -> Result<TantivyDocument> {
        todo!()
    }

    pub fn transform_calendar_event(
        &self,
        event: &CalendarEvent,
        channel: &Channel,
    ) -> Result<TantivyDocument> {
        todo!()
    }
}
