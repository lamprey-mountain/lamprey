use common::v1::types::{
    AuditLogEntryId, ChannelId, MediaId, MessageId, RoomId, UserId, util::Time,
};
use lamprey_backend_core::Error;
use tantivy::schema::{
    OwnedValue,
    document::{DeserializeError, DocumentDeserialize, DocumentDeserializer},
};
use time::OffsetDateTime;

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

pub struct TantivyMedia {
    pub id: MediaId,
}

pub struct TantivyUser {
    pub id: UserId,
}

pub struct TantivyRoom {
    pub id: RoomId,
}

pub struct TantivyAuditLogEntry {
    pub id: AuditLogEntryId,
    pub room_id: RoomId,
}

struct TantivyMessagePartial {
    id: Option<MessageId>,
    channel_id: Option<ChannelId>,
}

struct TantivyChannelPartial {
    id: Option<ChannelId>,
    archived_at: Option<Time>,
    created_at: Option<Time>,
}

struct TantivyMediaPartial {
    id: Option<MediaId>,
}

struct TantivyUserPartial {
    id: Option<UserId>,
}

struct TantivyRoomPartial {
    id: Option<RoomId>,
}

struct TantivyAuditLogEntryPartial {
    id: Option<AuditLogEntryId>,
    room_id: Option<RoomId>,
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

impl DocumentDeserialize for TantivyChannel {
    fn deserialize<'de, D>(mut deserializer: D) -> Result<Self, DeserializeError>
    where
        D: DocumentDeserializer<'de>,
    {
        let mut channel = TantivyChannelPartial {
            id: None,
            archived_at: None,
            created_at: None,
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
                    channel.id = Some(id);
                }
                "archived_at" => {
                    if let OwnedValue::Date(dt) = v {
                        channel.archived_at = Some(dt.into_utc().into());
                    }
                }
                "created_at" => {
                    if let OwnedValue::Date(dt) = v {
                        channel.created_at = Some(dt.into_utc().into());
                    }
                }
                _ => {}
            }
        }

        channel.try_into().map_err(DeserializeError::custom)
    }
}

impl DocumentDeserialize for TantivyMedia {
    fn deserialize<'de, D>(mut deserializer: D) -> Result<Self, DeserializeError>
    where
        D: DocumentDeserializer<'de>,
    {
        let mut media = TantivyMediaPartial { id: None };

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
                    media.id = Some(id);
                }
                _ => {}
            }
        }

        media.try_into().map_err(DeserializeError::custom)
    }
}

impl DocumentDeserialize for TantivyUser {
    fn deserialize<'de, D>(mut deserializer: D) -> Result<Self, DeserializeError>
    where
        D: DocumentDeserializer<'de>,
    {
        let mut user = TantivyUserPartial { id: None };

        while let Some((field, v)) = deserializer.next_field::<OwnedValue>()? {
            match SCHEMA.schema.get_field_name(field) {
                "id" => {
                    let id = match v {
                        OwnedValue::Str(s) => s,
                        _ => return Err(DeserializeError::custom("missing id")),
                    };
                    let id = id
                        .parse()
                        .map_err(|_| DeserializeError::custom("invalid user id"))?;
                    user.id = Some(id);
                }
                _ => {}
            }
        }

        user.try_into().map_err(DeserializeError::custom)
    }
}

impl DocumentDeserialize for TantivyRoom {
    fn deserialize<'de, D>(mut deserializer: D) -> Result<Self, DeserializeError>
    where
        D: DocumentDeserializer<'de>,
    {
        let mut room = TantivyRoomPartial { id: None };

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
                    room.id = Some(id);
                }
                _ => {}
            }
        }

        room.try_into().map_err(DeserializeError::custom)
    }
}

impl DocumentDeserialize for TantivyAuditLogEntry {
    fn deserialize<'de, D>(mut deserializer: D) -> Result<Self, DeserializeError>
    where
        D: DocumentDeserializer<'de>,
    {
        let mut entry = TantivyAuditLogEntryPartial {
            id: None,
            room_id: None,
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
                    entry.id = Some(id);
                }
                "room_id" => {
                    let id = match v {
                        OwnedValue::Str(s) => s,
                        _ => return Err(DeserializeError::custom("missing room_id")),
                    };
                    let id = id
                        .parse()
                        .map_err(|_| DeserializeError::custom("invalid uuid"))?;
                    entry.room_id = Some(id);
                }
                _ => {}
            }
        }

        entry.try_into().map_err(DeserializeError::custom)
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

impl TryFrom<TantivyChannelPartial> for TantivyChannel {
    type Error = Error;

    fn try_from(value: TantivyChannelPartial) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value
                .id
                .ok_or_else(|| Error::Internal("missing id".to_string()))?,
            archived_at: value.archived_at,
            // TODO: maybe handle missing created_at better?
            // i have no idea why some channels don't have created_at, i should probably debug that
            // though TantivyChannel.created_at isnt really used anyways? its only used in tantivy for sorting?
            created_at: value
                .created_at
                .unwrap_or_else(|| Time::from(OffsetDateTime::UNIX_EPOCH)),
        })
    }
}

impl TryFrom<TantivyMediaPartial> for TantivyMedia {
    type Error = Error;

    fn try_from(value: TantivyMediaPartial) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value
                .id
                .ok_or_else(|| Error::Internal("missing id".to_string()))?,
        })
    }
}

impl TryFrom<TantivyUserPartial> for TantivyUser {
    type Error = Error;

    fn try_from(value: TantivyUserPartial) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value
                .id
                .ok_or_else(|| Error::Internal("missing id".to_string()))?,
        })
    }
}

impl TryFrom<TantivyRoomPartial> for TantivyRoom {
    type Error = Error;

    fn try_from(value: TantivyRoomPartial) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value
                .id
                .ok_or_else(|| Error::Internal("missing id".to_string()))?,
        })
    }
}

impl TryFrom<TantivyAuditLogEntryPartial> for TantivyAuditLogEntry {
    type Error = Error;

    fn try_from(value: TantivyAuditLogEntryPartial) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value
                .id
                .ok_or_else(|| Error::Internal("missing id".to_string()))?,
            room_id: value
                .room_id
                .ok_or_else(|| Error::Internal("missing room_id".to_string()))?,
        })
    }
}
