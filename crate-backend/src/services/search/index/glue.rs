use common::v1::types::{util::Time, AuditLogEntryId, ChannelId, MessageId, RoomId};
use lamprey_backend_core::Error;
use tantivy::schema::{
    document::{DeserializeError, DocumentDeserialize, DocumentDeserializer},
    OwnedValue,
};

use crate::services::search::util::SCHEMA;

// TODO: maybe use macros for these
//
// #[derive(DocumentDeserialize)]
// pub struct TantivyMessage {
//     pub id: MessageId,
//     pub channel_id: ChannelId,
// }

// PERF: don't use OwnedValue
// TODO: use as_str. this should be part of tantivy's Value trait, but it doesn't seem to work.

/// a deserialized message document from tantivy
// TODO: use in place of SearchMessagesResponseRawItem
pub struct TantivyMessage {
    pub id: MessageId,
    pub channel_id: ChannelId,
}

// TODO: impl DocumentDeserialize
pub struct TantivyChannel {
    pub id: ChannelId,
    pub archived_at: Option<Time>,
    pub created_at: Time,
}

// TODO: impl DocumentDeserialize
pub struct TantivyRoom {
    pub id: RoomId,
}

// TODO: impl DocumentDeserialize
pub struct TantivyAuditLogEntry {
    pub id: AuditLogEntryId,
    pub room_id: RoomId,
}

struct TantivyMessagePartial {
    id: Option<MessageId>,
    channel_id: Option<ChannelId>,
}

impl DocumentDeserialize for TantivyMessage {
    fn deserialize<'de, D>(mut deserializer: D) -> Result<Self, DeserializeError>
    where
        D: DocumentDeserializer<'de>,
    {
        let mut msg = TantivyMessagePartial {
            id: None,
            channel_id: None,
        };

        while let Some((field, v)) = deserializer.next_field::<OwnedValue>()? {
            match SCHEMA.schema.get_field_name(field) {
                "id" => {
                    let id = match v {
                        OwnedValue::Str(s) => s,
                        _ => return Err(DeserializeError::custom("missing id")),
                    };
                    let id = id
                        .parse()
                        .map_err(|_| DeserializeError::custom("invalid uuid"))?;
                    msg.id = Some(id);
                }
                "channel_id" => {
                    let id = match v {
                        OwnedValue::Str(s) => s,
                        _ => return Err(DeserializeError::custom("missing id")),
                    };
                    let id = id
                        .parse()
                        .map_err(|_| DeserializeError::custom("invalid uuid"))?;
                    msg.channel_id = Some(id);
                }
                _ => {}
            }
        }

        msg.try_into().map_err(DeserializeError::custom)
    }
}

impl TryFrom<TantivyMessagePartial> for TantivyMessage {
    type Error = Error;

    fn try_from(value: TantivyMessagePartial) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value
                .id
                .ok_or_else(|| Error::Internal("missing id".to_string()))?,
            channel_id: value
                .channel_id
                .ok_or_else(|| Error::Internal("missing channel_id".to_string()))?,
        })
    }
}
