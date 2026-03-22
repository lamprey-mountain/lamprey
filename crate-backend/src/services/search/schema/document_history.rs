use tantivy::schema::{self, Schema, SchemaBuilder, FAST, STORED, STRING};

use crate::services::search::schema::IndexDefinition;

pub struct DocumentHistoryIndex;

pub struct DocumentHistorySchema {
    /// the tantivy schema itself
    pub schema: Schema,

    /// the author of this document history entry
    pub author_id: schema::Field,

    /// when this history entry was created at
    pub created_at: schema::Field,

    /// number of bytes added
    pub stat_added: schema::Field,

    /// number of bytes removed
    pub stat_removed: schema::Field,

    /// sequence number
    pub seq: schema::Field,

    /// the document ID this entry belongs to
    pub document_id: schema::Field,

    /// the branch ID this entry belongs to
    pub branch_id: schema::Field,
}

impl IndexDefinition for DocumentHistoryIndex {
    fn schema(&self) -> &Schema {
        &self.schema()
    }

    fn name(&self) -> String {
        "document_history".to_owned()
    }
}

impl DocumentHistorySchema {
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

impl Default for DocumentHistorySchema {
    fn default() -> Self {
        let mut sb = SchemaBuilder::new();

        let author_id = sb.add_text_field("author_id", STRING | FAST | STORED);
        let created_at = sb.add_date_field("created_at", FAST);
        let stat_added = sb.add_u64_field("stat_added", FAST);
        let stat_removed = sb.add_u64_field("stat_removed", FAST);
        let seq = sb.add_u64_field("seq", FAST);
        let document_id = sb.add_text_field("document_id", STRING | FAST | STORED);
        let branch_id = sb.add_text_field("branch_id", STRING | FAST | STORED);

        let schema = sb.build();

        Self {
            schema,
            author_id,
            created_at,
            stat_added,
            stat_removed,
            seq,
            document_id,
            branch_id,
        }
    }
}
