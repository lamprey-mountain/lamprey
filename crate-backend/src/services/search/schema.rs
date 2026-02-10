use common::v1::types::{Channel, Media, Message, MessageId, MessageType, Room, User};
use serde::{Deserialize, Serialize};
use serde_json;
use tantivy::{
    schema::{
        self, IndexRecordOption, JsonObjectOptions, Schema, SchemaBuilder, TextFieldIndexing,
        TextOptions, FAST, INDEXED, STORED, STRING,
    },
    tokenizer::Tokenizer,
    TantivyDocument,
};

/// tantivy schema for lamprey
#[derive(Debug, Clone)]
pub struct LampreySchema {
    /// the tantivy schema itself
    pub schema: Schema,

    /// the id of this object
    pub id: schema::Field,

    /// the type of this object
    pub doctype: schema::Field,

    /// when this object was created at
    pub created_at: schema::Field,

    /// when this object was updated/edited at
    pub updated_at: schema::Field,

    /// when this object was archived at
    pub archived_at: schema::Field,

    /// when this object was deleted at, for admins only.
    pub deleted_at: schema::Field,

    /// the author of this object
    ///
    /// - room owner_id
    /// - bot users owner_id
    /// - channel owner_id
    /// - message author_id
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

    /// if this message has an associated thread
    ///
    /// for messages
    pub has_thread: schema::Field,

    /// if this message is pinned
    ///
    /// for messages
    pub pinned: schema::Field,

    /// the message this is replying to
    ///
    /// for messages
    pub reply: schema::Field,

    /// the size of this media
    ///
    /// for media and messages (attachments)
    pub media_size: schema::Field,

    /// the content type of the media
    ///
    /// for media and messages (attachments)
    pub media_content_type: schema::Field,

    /// the alt text of the media
    ///
    /// for media and messages (attachments)
    pub media_alt: schema::Field,

    /// the filename of the media
    ///
    /// for media and messages (attachments)
    pub media_filename: schema::Field,

    /// if this thing is quarantined
    ///
    /// for rooms and media
    pub quarantined: schema::Field,

    /// if this message has an attachment
    ///
    /// for messages
    pub has_attachment: schema::Field,

    /// if this message has audio
    ///
    /// for messages
    pub has_audio: schema::Field,

    /// if this message has an embed
    ///
    /// for messages
    pub has_embed: schema::Field,

    /// if this message has an image
    ///
    /// for messages
    pub has_image: schema::Field,

    /// if this message has a link
    ///
    /// for messages
    pub has_link: schema::Field,

    /// if this message has a video
    ///
    /// for messages
    pub has_video: schema::Field,

    /// if this message mentions everyone
    ///
    /// for messages
    pub mentions_everyone: schema::Field,

    /// IDs of roles mentioned in this message
    ///
    /// for messages
    pub mentions_role: schema::Field,

    /// IDs of users mentioned in this message
    ///
    /// for messages
    pub mentions_user: schema::Field,

    /// hostname of links in this message
    ///
    /// for messages
    pub link_hostname: schema::Field,

    /// arbitrary json data in case i need to edit the schema
    pub metadata: schema::Field,
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

        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("dynamic")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        let id = sb.add_text_field("id", STRING | FAST | STORED);
        let doctype = sb.add_text_field("doctype", STRING | FAST);
        let created_at = sb.add_date_field("created_at", INDEXED | STORED);
        let updated_at = sb.add_date_field("updated_at", INDEXED | STORED);
        let archived_at = sb.add_date_field("archived_at", INDEXED | STORED);
        let deleted_at = sb.add_date_field("deleted_at", INDEXED | STORED);
        let author_id = sb.add_text_field("author_id", STRING);
        let channel_id = sb.add_text_field("channel_id", STRING | FAST);
        let room_id = sb.add_text_field("room_id", STRING | FAST);
        let tag_id = sb.add_text_field("tag_id", STRING);
        let name = sb.add_text_field("name", text_options.clone());
        let content = sb.add_text_field("content", text_options.clone());
        let has_thread = sb.add_bool_field("has_thread", INDEXED);
        let pinned = sb.add_bool_field("pinned", INDEXED);
        let reply = sb.add_text_field("reply", STRING);
        let media_size = sb.add_u64_field("media_size", INDEXED);
        let media_content_type = sb.add_text_field("media_content_type", STRING);
        let media_alt = sb.add_text_field("media_alt", text_options.clone());
        let media_filename = sb.add_text_field("media_filename", STRING);
        let quarantined = sb.add_bool_field("quarantined", INDEXED);
        let has_attachment = sb.add_bool_field("has_attachment", INDEXED);
        let has_audio = sb.add_bool_field("has_audio", INDEXED);
        let has_embed = sb.add_bool_field("has_embed", INDEXED);
        let has_image = sb.add_bool_field("has_image", INDEXED);
        let has_link = sb.add_bool_field("has_link", INDEXED);
        let has_video = sb.add_bool_field("has_video", INDEXED);
        let mentions_everyone = sb.add_bool_field("mentions_everyone", INDEXED);
        let mentions_role = sb.add_text_field("mentions_role", STRING);
        let mentions_user = sb.add_text_field("mentions_user", STRING);
        let link_hostname = sb.add_text_field("link_hostname", STRING);

        let metadata = sb.add_json_field(
            "metadata",
            JsonObjectOptions::default().set_indexing_options(TextFieldIndexing::default()),
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
            author_id,
            channel_id,
            room_id,
            tag_id,
            name,
            content,
            has_thread,
            pinned,
            reply,
            media_size,
            media_content_type,
            media_alt,
            media_filename,
            quarantined,
            has_attachment,
            has_audio,
            has_embed,
            has_image,
            has_link,
            has_video,
            mentions_everyone,
            mentions_role,
            mentions_user,
            link_hostname,
            metadata,
        }
    }
}

/// creaet a tantivy document from a message
pub fn tantivy_document_from_message(s: &LampreySchema, message: Message) -> TantivyDocument {
    let mut doc = TantivyDocument::new();
    doc.add_text(s.id, message.id.to_string());
    doc.add_text(s.channel_id, message.channel_id.to_string());
    doc.add_text(s.author_id, message.author_id.to_string());
    doc.add_date(
        s.created_at,
        tantivy::DateTime::from_utc(*message.created_at),
    );

    let reply = match &message.latest_version.message_type {
        MessageType::DefaultMarkdown(m) => m.reply_id,
        MessageType::MessagePinned(p) => Some(p.pinned_message_id), // not *technically* correct, but still useful
        MessageType::ThreadCreated(m) => m.source_message_id, // not *technically* correct, but still useful
        _ => None,
    };

    let mn = &message.latest_version.mentions;

    // TODO: index all messages, not just DefaultMarkdown
    match message.latest_version.message_type {
        MessageType::DefaultMarkdown(m) => {
            if let Some(c) = &m.content {
                doc.add_text(s.content, c);
            }

            // Add individual fields instead of JSON metadata
            doc.add_bool(s.has_thread, message.thread.is_some());
            doc.add_bool(s.pinned, message.pinned.is_some());

            if let Some(reply_id) = reply {
                doc.add_text(s.reply, reply_id.to_string());
            }

            // Process attachments to populate media-related fields
            if !m.attachments.is_empty() {
                // Add attachment-related flags
                doc.add_bool(s.has_attachment, true);

                // Check for different media types
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

                doc.add_bool(s.has_audio, has_audio);
                doc.add_bool(s.has_image, has_image);
                doc.add_bool(s.has_video, has_video);

                // Add details for the first attachment as examples (in a real implementation, you might want to handle multiple attachments differently)
                if let Some(first_attachment) = m.attachments.first() {
                    doc.add_u64(s.media_size, first_attachment.source.size);
                    doc.add_text(
                        s.media_content_type,
                        first_attachment.source.mime.to_string(),
                    );

                    if let Some(alt) = &first_attachment.alt {
                        doc.add_text(s.media_alt, alt.clone());
                    }

                    doc.add_text(s.media_filename, first_attachment.filename.clone());
                }
            } else {
                doc.add_bool(s.has_attachment, false);
                doc.add_bool(s.has_audio, false);
                doc.add_bool(s.has_image, false);
                doc.add_bool(s.has_video, false);
            }

            // Add embed-related flag
            doc.add_bool(s.has_embed, !m.embeds.is_empty());

            // Add mention-related fields
            doc.add_bool(s.mentions_everyone, mn.everyone);

            if !mn.roles.is_empty() {
                let role_ids: Vec<String> = mn.roles.iter().map(|r| r.id.to_string()).collect();
                // For simplicity, we'll store as a joined string; in practice, you might want to add multiple values
                doc.add_text(s.mentions_role, role_ids.join(","));
            }

            if !mn.users.is_empty() {
                let user_ids: Vec<String> = mn.users.iter().map(|u| u.id.to_string()).collect();
                // For simplicity, we'll store as a joined string; in practice, you might want to add multiple values
                doc.add_text(s.mentions_user, user_ids.join(","));
            }

            // Add link-related fields (placeholder - would need actual link detection)
            doc.add_bool(s.has_link, false); // TODO: implement link detection
                                             // doc.add_text(s.link_hostname, ""); // TODO: implement link hostname extraction
        }
        _ => {}
    };
    doc
}

pub fn tantivy_document_from_user(user: User) -> TantivyDocument {
    todo!()
}

pub fn tantivy_document_from_room(user: Room) -> TantivyDocument {
    todo!()
}

pub fn tantivy_document_from_channel(user: Channel) -> TantivyDocument {
    todo!()
}

pub fn tantivy_document_from_media(user: Media) -> TantivyDocument {
    todo!()
}
