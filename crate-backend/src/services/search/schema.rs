use std::collections::BTreeMap;
use std::path::Path;

use common::v1::types::util::Time;
use common::v1::types::{Channel, ChannelType, Message, MessageType, Room, User};
use common::v2::types::media::Media;
use serde::{Deserialize, Serialize};
use tantivy::schema::TextOptions;
use tantivy::schema::{
    self, IndexRecordOption, JsonObjectOptions, OwnedValue, Schema, SchemaBuilder,
    TextFieldIndexing, FAST, STORED, STRING,
};
use tantivy::TantivyDocument;

/// tantivy schema for lamprey
#[derive(Debug, Clone)]
pub struct LampreySchema {
    /// the tantivy schema itself
    pub schema: Schema,

    /// the id of this object
    pub id: schema::Field,

    /// the type of this object
    pub doctype: schema::Field,

    /// subtype for storing message type and channel type
    ///
    /// for messages and channels
    pub subtype: schema::Field,

    /// when this object was created at
    pub created_at: schema::Field,

    /// when this object was updated/edited at
    pub updated_at: schema::Field,

    /// when this object was archived at
    pub archived_at: schema::Field,

    /// when this object was deleted at, for admins only.
    pub deleted_at: schema::Field,

    /// when this object was removed at, for moderators only.
    pub removed_at: schema::Field,

    /// the author of this object
    ///
    /// - room owner_id
    /// - bot users owner_id
    /// - channel owner_id
    /// - message author_id
    /// - media user_id
    // NOTE: for channels, author_id exists, but owner_id could be better in the case of gdms
    pub author_id: schema::Field,

    /// the channel this object is in
    ///
    /// - threads: the channel's parent_id
    /// - message: the message's channel_id
    pub channel_id: schema::Field,

    /// the channel this object is in
    ///
    /// for channels, threads, messages, media
    pub room_id: schema::Field,

    /// the tags this object has
    ///
    /// for threads
    pub tag_id: schema::Field,

    /// the name of this object
    ///
    /// - room, channel, user name
    /// - (empty for message and media)
    pub name: schema::Field,

    /// the main content of this object
    ///
    /// - room description
    /// - channel topic
    /// - user bio
    /// - message content
    pub content: schema::Field,

    /// fast metadata for filtering and sorting
    ///
    /// contains booleans, numbers, keywords (IDs)
    pub metadata_fast: schema::Field,

    /// text metadata for full-text search
    ///
    /// contains natural language text (alt text, etc.)
    pub metadata_text: schema::Field,
}

/// the type of this item
#[derive(Debug, Serialize, Deserialize)]
pub enum LampreySchemaDoctype {
    Message,
    Channel,
    Room,
    User,
    Media,
    // TODO: more searching
    // Document, // branch_id, template, draft, published, view_count(?)(sorting)
    // Tag, // usage_count(sorting)
    // Application, // public(admin only), usage_count(sorting)
    // CalendarEvent, // location, starts_at, ends_at
    // RoomTemplate, // usage_count(sorting)
    // Emoji, // animated, usage_count(sorting)
    // Broadcasts, // member_count(sorting)
}

impl Default for LampreySchema {
    fn default() -> Self {
        let mut sb = SchemaBuilder::new();

        // config for human readable text
        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("dynamic")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        let id = sb.add_text_field("id", STRING | FAST | STORED);
        let doctype = sb.add_text_field("doctype", STRING | FAST | STORED);
        let subtype = sb.add_text_field("subtype", STRING | FAST);
        let created_at = sb.add_date_field("created_at", FAST);
        let updated_at = sb.add_date_field("updated_at", FAST);
        let archived_at = sb.add_date_field("archived_at", FAST);
        let deleted_at = sb.add_date_field("deleted_at", FAST);
        let removed_at = sb.add_date_field("removed_at", FAST);
        let author_id = sb.add_text_field("author_id", STRING | FAST);
        let channel_id = sb.add_text_field("channel_id", STRING | FAST | STORED);
        let room_id = sb.add_text_field("room_id", STRING | FAST | STORED);
        let tag_id = sb.add_text_field("tag_id", STRING | FAST);
        let name = sb.add_text_field("name", text_options.clone()); // TODO: boost
        let content = sb.add_text_field("content", text_options.clone());

        let metadata_fast =
            sb.add_json_field("metadata_fast", JsonObjectOptions::default().set_fast(None));

        let metadata_text = sb.add_json_field(
            "metadata_text",
            JsonObjectOptions::default().set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("dynamic")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            ),
        );

        let schema = sb.build();

        Self {
            schema,
            id,
            doctype,
            created_at,
            updated_at,
            archived_at,
            deleted_at,
            removed_at,
            author_id,
            channel_id,
            room_id,
            tag_id,
            name,
            content,
            metadata_fast,
            metadata_text,
            subtype,
        }
    }
}

/// create a tantivy document from a message
pub fn tantivy_document_from_message(
    s: &LampreySchema,
    message: Message,
    room_id: Option<common::v1::types::RoomId>,
) -> TantivyDocument {
    let mut doc = TantivyDocument::new();
    doc.add_text(s.id, message.id.to_string());
    doc.add_text(s.doctype, "Message");
    doc.add_text(s.channel_id, message.channel_id.to_string());
    doc.add_text(s.author_id, message.author_id.to_string());
    doc.add_date(
        s.created_at,
        tantivy::DateTime::from_utc(*message.created_at),
    );

    let updated_at = message.latest_version.created_at;
    if updated_at != message.created_at {
        doc.add_date(s.updated_at, tantivy::DateTime::from_utc(*updated_at));
    }

    if let Some(deleted_at) = message.deleted_at {
        doc.add_date(s.deleted_at, tantivy::DateTime::from_utc(*deleted_at));
    }

    if let Some(removed_at) = message.removed_at {
        doc.add_date(s.removed_at, tantivy::DateTime::from_utc(*removed_at));
    }

    if let Some(room_id) = room_id {
        doc.add_text(s.room_id, room_id.to_string());
    }

    doc.add_text(
        s.subtype,
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

    // get what this message is "replying" to
    let reply = match &message.latest_version.message_type {
        MessageType::DefaultMarkdown(m) => m.reply_id,

        // these are not *technically* correct, but still useful
        MessageType::MessagePinned(p) => Some(p.pinned_message_id),
        MessageType::ThreadCreated(m) => m.source_message_id,
        _ => None,
    };

    doc.add_text(s.content, message.latest_version.message_type.to_string());

    if let MessageType::DefaultMarkdown(ref m) = message.latest_version.message_type {
        if !m.attachments.is_empty() {
            meta_fast.insert("has_attachment".to_string(), true.into());

            let has_audio = m
                .attachments
                .iter()
                .any(|a| a.source.mime.starts_with("audio/"));
            let has_image = m
                .attachments
                .iter()
                .any(|a| a.source.mime.starts_with("image/"));
            let has_video = m
                .attachments
                .iter()
                .any(|a| a.source.mime.starts_with("video/"));

            meta_fast.insert("has_audio".to_string(), has_audio.into());
            meta_fast.insert("has_image".to_string(), has_image.into());
            meta_fast.insert("has_video".to_string(), has_video.into());

            for att in &m.attachments {
                // Helper to push to array
                let push_val =
                    |map: &mut BTreeMap<String, OwnedValue>, key: &str, val: OwnedValue| {
                        let entry = map
                            .entry(key.to_string())
                            .or_insert_with(|| OwnedValue::Array(Vec::new()));
                        if let OwnedValue::Array(vec) = entry {
                            vec.push(val);
                        }
                    };

                push_val(&mut meta_fast, "media_size", att.source.size.into());
                push_val(
                    &mut meta_fast,
                    "media_content_type",
                    att.source.mime.as_str().into(),
                );
                push_val(
                    &mut meta_fast,
                    "media_filename",
                    att.filename.as_str().into(),
                );

                if let Some(alt) = &att.alt {
                    push_val(&mut meta_text, "media_alt", alt.as_str().into());
                }

                let extension = Path::new(&att.filename)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| ext.to_lowercase());
                if let Some(e) = extension {
                    push_val(&mut meta_fast, "media_extension", e.as_str().into());
                }
            }
        } else {
            // shortcut for messages with no attachments
            meta_fast.insert("has_attachment".to_string(), false.into());
            meta_fast.insert("has_audio".to_string(), false.into());
            meta_fast.insert("has_image".to_string(), false.into());
            meta_fast.insert("has_video".to_string(), false.into());
        }

        meta_fast.insert("has_embed".to_string(), (!m.embeds.is_empty()).into());
    };

    // common fields for all message types
    meta_fast.insert("has_thread".to_string(), message.thread.is_some().into());
    meta_fast.insert("pinned".to_string(), message.pinned.is_some().into());

    if let Some(reply_id) = reply {
        meta_fast.insert("reply".to_string(), reply_id.to_string().into());
    }

    // add mention fields
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

    // link fields
    let mut has_links = false;
    if let MessageType::DefaultMarkdown(ref m) = message.latest_version.message_type {
        if let Some(ref content) = m.content {
            let finder = linkify::LinkFinder::new();
            let mut hostnames = Vec::new();
            for link in finder.links(content) {
                if let Ok(url) = url::Url::parse(link.as_str()) {
                    if let Some(host) = url.host_str() {
                        // reverse the hostname (e.g., "foobar.example.com" -> "com.example.foobar")
                        // this is so that searching "example.com" can return results for "foobar.example.com" if needed
                        let reversed_hostname = host.split('.').rev().collect::<Vec<_>>().join(".");
                        hostnames.push(reversed_hostname.into());
                        has_links = true;
                    }
                }
            }
            if !hostnames.is_empty() {
                meta_fast.insert("link_hostname".to_string(), OwnedValue::Array(hostnames));
            }
        }
    }
    meta_fast.insert("has_link".to_string(), has_links.into());

    doc.add_object(s.metadata_fast, meta_fast);
    doc.add_object(s.metadata_text, meta_text);

    doc
}

pub fn _tantivy_document_from_user(_user: User) -> TantivyDocument {
    todo!()
}

pub fn _tantivy_document_from_room(_room: Room) -> TantivyDocument {
    todo!()
}

pub fn tantivy_document_from_channel(s: &LampreySchema, channel: Channel) -> TantivyDocument {
    let mut doc = TantivyDocument::new();
    doc.add_text(s.id, channel.id.to_string());
    doc.add_text(s.doctype, "Channel");
    doc.add_text(s.name, channel.name);

    if let Some(description) = channel.description {
        doc.add_text(s.content, description);
    }

    if let Some(room_id) = channel.room_id {
        doc.add_text(s.room_id, room_id.to_string());
    }

    if let Some(parent_id) = channel.parent_id {
        doc.add_text(s.channel_id, parent_id.to_string());
    }

    if let Some(owner_id) = channel.owner_id.map(|i| i.to_string()) {
        doc.add_text(s.author_id, owner_id);
    }

    if let Some(tags) = &channel.tags {
        for tag_id in tags {
            doc.add_text(s.tag_id, tag_id.to_string());
        }
    }

    let created_at: Time = channel.id.try_into().unwrap();
    doc.add_date(s.created_at, tantivy::DateTime::from_utc(*created_at));

    if let Some(deleted_at) = channel.deleted_at {
        doc.add_date(s.deleted_at, tantivy::DateTime::from_utc(*deleted_at));
    }

    if let Some(archived_at) = channel.archived_at {
        doc.add_date(s.archived_at, tantivy::DateTime::from_utc(*archived_at));
    }

    doc.add_text(
        s.subtype,
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
        },
    );

    let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
    let mut meta_text: BTreeMap<String, OwnedValue> = BTreeMap::new();

    meta_fast.insert("nsfw".to_string(), channel.nsfw.into());

    if let Some(bitrate) = channel.bitrate {
        meta_fast.insert("bitrate".to_string(), bitrate.into());
    }

    if let Some(user_limit) = channel.user_limit {
        meta_fast.insert("user_limit".to_string(), user_limit.into());
    }

    doc.add_object(s.metadata_fast, meta_fast);
    doc.add_object(s.metadata_text, meta_text);

    doc
}

pub fn _tantivy_document_from_media(s: &LampreySchema, media: Media) -> TantivyDocument {
    let mut doc = TantivyDocument::new();

    doc.add_text(s.id, media.id.to_string());
    doc.add_text(s.doctype, "Media");

    let created_at: Time = media.id.try_into().unwrap();
    doc.add_date(s.created_at, tantivy::DateTime::from_utc(*created_at));
    doc.add_text(
        s.author_id,
        media
            .user_id
            .as_ref()
            .expect("the server should always have user_id")
            .to_string(),
    );

    if let Some(r) = media.room_id {
        doc.add_text(s.room_id, r.to_string());
    }

    if let Some(r) = media.channel_id {
        doc.add_text(s.channel_id, r.to_string());
    }

    let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
    let mut meta_text: BTreeMap<String, OwnedValue> = BTreeMap::new();

    meta_fast.insert("media_size".to_string(), media.size.into());
    meta_fast.insert(
        "media_content_type".to_string(),
        media.content_type.to_string().into(),
    );
    meta_fast.insert("media_filename".to_string(), media.filename.clone().into());

    let extension = Path::new(&media.filename)
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

    doc.add_object(s.metadata_fast, meta_fast);
    doc.add_object(s.metadata_text, meta_text);

    doc
}
