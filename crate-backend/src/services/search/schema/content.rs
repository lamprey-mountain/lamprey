use serde::{Deserialize, Serialize};
use tantivy::schema::{
    self, IndexRecordOption, JsonObjectOptions, Schema, SchemaBuilder, TextFieldIndexing,
    TextOptions, FAST, STORED, STRING, TEXT,
};

use super::IndexDefinition;

#[derive(Default)]
pub struct ContentIndex {
    schema: ContentSchema,
}

impl IndexDefinition for ContentIndex {
    fn schema(&self) -> &Schema {
        &self.schema.schema
    }

    fn name(&self) -> String {
        "content".to_owned()
    }
}

/// tantivy schema for lamprey content
#[derive(Debug, Clone)]
pub struct ContentSchema {
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

    /// the parent channel of the channel this object is in
    pub parent_channel_id: schema::Field,

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
pub enum ContentSchemaDoctype {
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

impl Default for ContentSchema {
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
        let doctype = sb.add_text_field("doctype", STRING | FAST | STORED);
        let subtype = sb.add_text_field("subtype", STRING | FAST);
        let created_at = sb.add_date_field("created_at", FAST);
        let updated_at = sb.add_date_field("updated_at", FAST);
        let archived_at = sb.add_date_field("archived_at", FAST);
        let deleted_at = sb.add_date_field("deleted_at", FAST);
        let removed_at = sb.add_date_field("removed_at", FAST);
        let author_id = sb.add_text_field("author_id", STRING | FAST);
        let channel_id = sb.add_text_field("channel_id", STRING | FAST | STORED);
        let parent_channel_id = sb.add_text_field("parent_channel_id", STRING | FAST | STORED);
        let room_id = sb.add_text_field("room_id", STRING | FAST | STORED);
        let tag_id = sb.add_text_field("tag_id", STRING | FAST);
        let name = sb.add_text_field("name", text_options.clone());
        let content = sb.add_text_field("content", text_options.clone());

        let metadata_fast = sb.add_json_field(
            "metadata_fast",
            JsonObjectOptions::default()
                .set_fast(None)
                .set_indexing_options(
                    TextFieldIndexing::default()
                        .set_tokenizer("raw")
                        .set_index_option(IndexRecordOption::Basic),
                ),
        );

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
            parent_channel_id,
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
