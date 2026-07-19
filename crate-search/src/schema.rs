use once_cell::sync::Lazy;
use tantivy::schema::{
    self, FAST, IndexRecordOption, JsonObjectOptions, STORED, STRING, Schema, SchemaBuilder, TEXT,
    TextFieldIndexing, TextOptions,
};

pub static SCHEMA: Lazy<UnifiedSchema> = Lazy::new(|| UnifiedSchema::default());

#[derive(Debug, Clone)]
pub struct UnifiedSchema {
    /// the tantivy schema itself
    pub schema: Schema,

    /// the id of this object
    pub id: schema::Field, // text(STRING | FAST | STORED)

    /// the type of this object
    pub doctype: schema::Field, // text(STRING | FAST | STORED)

    /// the subtype of this object (eg. message type, channel type)
    pub subtype: schema::Field, // text(STRING | FAST | STORED)

    // date fields
    /// when this item was created
    pub created_at: schema::Field,

    /// when this item was last updated
    ///
    /// only updates when the item itself updates. eg. for channels, this does not update when sending a message.
    pub updated_at: schema::Field,

    /// when this item was archived
    ///
    /// for audit log entries, this is the `ended_at` field
    pub archived_at: schema::Field,
    pub deleted_at: schema::Field,
    pub removed_at: schema::Field,

    /// the last activity of this channel
    ///
    /// used for sorting by `Activity`
    pub activity_at: schema::Field,

    // id fields
    /// the id of the user who created this
    ///
    /// - for users, this is the `author_id` field
    /// - for rooms, this is the `owner_id` field
    /// - for channels, this is the `owner_id` field
    pub author_id: schema::Field,

    /// the channel this item is in
    ///
    /// for channels, this is the parent id
    pub channel_id: schema::Field,

    /// for messages, the channel's parent channel id
    pub parent_channel_id: schema::Field,

    /// the room this item is in
    pub room_id: schema::Field,

    pub tag_id: schema::Field,
    pub branch_id: schema::Field, // document history

    // full text search
    pub name: schema::Field,
    pub content: schema::Field,

    // analytics (room analytics, abuse monitoring, audit logs)
    pub ip_addr: schema::Field,
    pub user_agent: schema::Field,
    pub session_id: schema::Field,
    pub application_id: schema::Field,
    pub audit_event: schema::Field,
    pub audit_reason: schema::Field,
    pub audit_status: schema::Field, // success, unauthorized, failed

    // document history and room analytics
    pub stat_added: schema::Field,   // u64(FAST)
    pub stat_removed: schema::Field, // u64(FAST)

    /// fast metadata for filtering and sorting
    ///
    /// contains booleans, numbers, keywords (IDs)
    // uses raw tokenizer
    pub metadata_fast: schema::Field,

    /// text metadata for full-text search
    ///
    /// contains natural language text (alt text, etc.)
    // uses dynamic tokenizer
    pub metadata_text: schema::Field,
}

impl Default for UnifiedSchema {
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
        let activity_at = sb.add_date_field("activity_at", FAST);
        let author_id = sb.add_text_field("author_id", STRING | FAST);
        let channel_id = sb.add_text_field("channel_id", STRING | FAST | STORED);
        let parent_channel_id = sb.add_text_field("parent_channel_id", STRING | FAST | STORED);
        let room_id = sb.add_text_field("room_id", STRING | FAST | STORED);
        let tag_id = sb.add_text_field("tag_id", STRING | FAST);
        let branch_id = sb.add_text_field("branch_id", STRING | FAST);
        let name = sb.add_text_field("name", text_options.clone());
        let content = sb.add_text_field("content", text_options.clone());
        let ip_addr = sb.add_ip_addr_field("ip_addr", FAST);
        let user_agent = sb.add_text_field("user_agent", TEXT | STORED);
        let session_id = sb.add_text_field("session_id", STRING | FAST);
        let application_id = sb.add_text_field("application_id", STRING | FAST);
        let audit_event = sb.add_text_field("audit_event", STRING | FAST);
        let audit_reason = sb.add_text_field("audit_reason", STRING | FAST);
        let audit_status = sb.add_text_field("audit_status", STRING | FAST);
        let stat_added = sb.add_u64_field("stat_added", FAST);
        let stat_removed = sb.add_u64_field("stat_removed", FAST);

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
            subtype,
            created_at,
            updated_at,
            archived_at,
            deleted_at,
            removed_at,
            activity_at,
            author_id,
            channel_id,
            parent_channel_id,
            room_id,
            tag_id,
            branch_id,
            name,
            content,
            ip_addr,
            user_agent,
            session_id,
            application_id,
            audit_event,
            audit_reason,
            audit_status,
            stat_added,
            stat_removed,
            metadata_fast,
            metadata_text,
        }
    }
}

// utilities for making constructing queries a bit easier
// TODO
#[cfg(any())]
impl UnifiedSchema {
    /// construct a term that requires `id` to match the given uuid
    pub fn term_id(&self, id: Uuid) -> Term {
        Term::from_field_text(self.id, &id.to_string())
    }

    /// construct a term query that requires `id` to match the given uuid
    pub fn query_id(&self, id: Uuid) -> Box<dyn Query> {
        Box::new(TermQuery::new(self.term_id(id), IndexRecordOption::Basic))
    }

    /// construct a term that requires `author_id` to match the given user id
    pub fn term_author_id(&self, author_id: UserId) -> Term {
        Term::from_field_text(self.author_id, &author_id.to_string())
    }

    /// construct a term query that requires `author_id` to match the given user id
    pub fn query_author_id(&self, author_id: UserId) -> Box<dyn Query> {
        Box::new(TermQuery::new(
            self.term_author_id(author_id),
            IndexRecordOption::Basic,
        ))
    }

    /// construct a term that requires `room_id` to match the given room id
    pub fn term_room_id(&self, room_id: RoomId) -> Term {
        Term::from_field_text(self.room_id, &room_id.to_string())
    }

    /// construct a term query that requires `room_id` to match the given room id
    pub fn query_room_id(&self, room_id: RoomId) -> Box<dyn Query> {
        Box::new(TermQuery::new(
            self.term_room_id(room_id),
            IndexRecordOption::Basic,
        ))
    }

    /// construct a term that requires `channel_id` to match the given channel id
    pub fn term_channel_id(&self, channel_id: ChannelId) -> Term {
        Term::from_field_text(self.channel_id, &channel_id.to_string())
    }

    /// construct a term query that requires `channel_id` to match the given channel id
    pub fn query_channel_id(&self, channel_id: ChannelId) -> Box<dyn Query> {
        Box::new(TermQuery::new(
            self.term_channel_id(channel_id),
            IndexRecordOption::Basic,
        ))
    }

    /// construct a term that requires `parent_channel_id` to match the given channel id
    pub fn term_parent_channel_id(&self, parent_channel_id: ChannelId) -> Term {
        Term::from_field_text(self.parent_channel_id, &parent_channel_id.to_string())
    }

    /// construct a term query that requires `parent_channel_id` to match the given channel id
    pub fn query_parent_channel_id(&self, parent_channel_id: ChannelId) -> Box<dyn Query> {
        Box::new(TermQuery::new(
            self.term_parent_channel_id(parent_channel_id),
            IndexRecordOption::Basic,
        ))
    }

    /// construct a term that requires `doctype` to match the given doctype
    pub fn term_doctype(&self, doctype: Doctype) -> Term {
        Term::from_field_text(self.doctype, doctype.as_str())
    }

    /// construct a term query that requires `doctype` to match the given doctype
    pub fn query_doctype(&self, doctype: Doctype) -> Box<dyn Query> {
        Box::new(TermQuery::new(
            self.term_doctype(doctype),
            IndexRecordOption::Basic,
        ))
    }

    /// construct a term that requires `metadata_fast.public` to exist and be true
    pub fn term_public(&self) -> Term {
        // FIXME: actually check that it equals `true`
        Term::from_field_json_path(self.metadata_fast, "public", false)
    }

    /// construct a term query that requires `metadata_fast.public` to exist and be true
    pub fn query_public(&self) -> Box<dyn Query> {
        Box::new(TermQuery::new(self.term_public(), IndexRecordOption::Basic))
    }
}
