use tantivy::schema::{
    self, IndexRecordOption, JsonObjectOptions, Schema, SchemaBuilder, TextFieldIndexing,
    TextOptions, FAST, STORED, STRING, TEXT,
};

use crate::services::search::schema::IndexDefinition;

/// an index containing any lamprey data type
#[derive(Default)]
pub struct UnifiedIndex {
    schema: UnifiedSchema,
}

impl IndexDefinition for UnifiedIndex {
    fn schema(&self) -> &Schema {
        &self.schema.schema
    }

    fn name(&self) -> String {
        "unified".to_owned()
    }
}

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
    pub created_at: schema::Field,
    pub updated_at: schema::Field,
    pub archived_at: schema::Field, // is ended_at for audit log entry
    pub deleted_at: schema::Field,
    pub removed_at: schema::Field,
    pub activity_at: schema::Field, // used for last activity sorting

    // id fields
    pub author_id: schema::Field,
    pub channel_id: schema::Field,
    pub parent_channel_id: schema::Field,
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
