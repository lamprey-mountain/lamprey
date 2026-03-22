use common::v1::types::{ChannelId, RoomId, RoomMemberOrigin, SessionId, UserId};
use lamprey_backend_core::types::analytics::{AbuseMetadata, AnalyticsEventPayload};
use time::Time;
use uuid::Uuid;

use tantivy::schema::{self, Schema, SchemaBuilder, FAST, STORED, STRING, TEXT};

use crate::services::search::schema::IndexDefinition;

pub struct AbuseMonitoringIndex {
    schema: AbuseMonitoringSchema,
}

pub struct AbuseMonitoringSchema {
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

    /// IP address of the request
    pub ip_addr: schema::Field,

    /// user agent of the request
    pub user_agent: schema::Field,

    /// session ID
    pub session_id: schema::Field,
}

/// a single abuse monitoring event
pub struct AbuseEvent {
    pub id: Uuid,
    /// the room id this happened in, or SERVER_ROOM_ID otherwise
    pub room_id: RoomId,
    pub time: Time,
    pub payload: AnalyticsEventPayload,
    pub abuse_metadata: Option<AbuseMetadata>,
}

impl IndexDefinition for AbuseMonitoringIndex {
    fn schema(&self) -> &Schema {
        &self.schema.schema
    }

    fn name(&self) -> String {
        "abuse_monitoring".to_owned()
    }
}

impl Default for AbuseMonitoringIndex {
    fn default() -> Self {
        Self {
            schema: AbuseMonitoringSchema::default(),
        }
    }
}

impl Default for AbuseMonitoringSchema {
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
        let ip_addr = sb.add_ip_addr_field("ip_addr", FAST);
        let user_agent = sb.add_text_field("user_agent", TEXT | STORED);
        let session_id = sb.add_text_field("session_id", STRING | FAST);

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
            ip_addr,
            user_agent,
            session_id,
        }
    }
}
