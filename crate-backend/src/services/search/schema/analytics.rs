use common::v1::types::RoomId;
use lamprey_backend_core::types::analytics::AnalyticsEventPayload;
use time::Time;
use uuid::Uuid;

use tantivy::schema::{self, Schema, SchemaBuilder, FAST, STORED, STRING};

use crate::services::search::schema::IndexDefinition;

pub struct AnalyticsIndex {
    schema: AnalyticsSchema,
}

pub struct AnalyticsSchema {
    /// the tantivy schema itself
    pub schema: Schema,

    /// unique event ID
    pub id: schema::Field,

    /// the room this event belongs to
    pub room_id: schema::Field,

    /// the user ID (if applicable)
    pub user_id: schema::Field,

    /// when this event occurred
    pub created_at: schema::Field,

    /// the type of event
    pub event_kind: schema::Field,

    /// the action (create, update, delete)
    pub action: schema::Field,

    /// the channel this event belongs to (if applicable)
    pub channel_id: schema::Field,

    /// whether the action was successful
    pub success: schema::Field,

    /// count for aggregated events
    pub count: schema::Field,
}

impl IndexDefinition for AnalyticsIndex {
    fn schema(&self) -> &Schema {
        &self.schema.schema
    }

    fn name(&self) -> String {
        "room_analytics".to_owned()
    }
}

impl Default for AnalyticsIndex {
    fn default() -> Self {
        Self {
            schema: AnalyticsSchema::default(),
        }
    }
}

impl Default for AnalyticsSchema {
    fn default() -> Self {
        let mut sb = SchemaBuilder::new();

        let id = sb.add_text_field("id", STRING | STORED);
        let room_id = sb.add_text_field("room_id", STRING | FAST | STORED);
        let user_id = sb.add_text_field("user_id", STRING | FAST);
        let created_at = sb.add_date_field("created_at", FAST);
        let event_kind = sb.add_text_field("event_kind", STRING | FAST);
        let action = sb.add_text_field("action", STRING | FAST);
        let channel_id = sb.add_text_field("channel_id", STRING | FAST);
        let success = sb.add_bool_field("success", FAST);
        let count = sb.add_u64_field("count", FAST);

        let schema = sb.build();

        Self {
            schema,
            id,
            room_id,
            user_id,
            created_at,
            event_kind,
            action,
            channel_id,
            success,
            count,
        }
    }
}
