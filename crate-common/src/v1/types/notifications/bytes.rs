//! notification binary encoding

use crate::v1::types::{
    notifications::{Notification, NotificationType},
    ChannelId, MessageId, NotificationId, SessionId,
};

/// serialized notification payload, sent through web push
#[derive(Debug, Clone)]
pub struct NotificationBytes {
    pub version: NotificationBytesVersion,
}

#[derive(Debug, Clone)]
pub enum NotificationBytesVersion {
    // 0x00
    V1 {
        notification_id: NotificationId,
        session_id: SessionId,
        flags: NotificationBytesFlags,
        ty: NotificationBytesType,
    },
}

#[derive(Debug, Clone)]
pub enum NotificationBytesType {
    // 0x00
    Message {
        channel_id: ChannelId,
        message_id: MessageId,
    },

    // 0x01
    Reaction {
        channel_id: ChannelId,
        message_id: MessageId,
        // TODO: add reaction_key
        // not added yet because main Notification struct doesnt have it yet
    },
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NotificationBytesFlags: u8 {
        /// the resource was edited
        // NOTE: not sure what the logic is for sending notifs for edited stuff
        const EDIT = 1 << 0;

        /// the author is ignored
        // NOTE: notifs from ignored users should not be sent to begin with?
        const AUTHOR_IGNORED = 1 << 1;

        /// the author is blocked
        // NOTE: notifs from blocked users should not be sent to begin with?
        const AUTHOR_BLOCKED = 1 << 2;

        /// the channel is muted
        // NOTE: notifs from muted channels should not be sent to begin with?
        const CHANNEL_MUTED = 1 << 3;
    }
}

impl NotificationBytesType {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            NotificationBytesType::Message {
                channel_id,
                message_id,
            } => {
                let mut bytes = Vec::with_capacity(1 + 16 + 16);
                bytes.push(0x00);
                bytes.extend_from_slice(channel_id.as_bytes());
                bytes.extend_from_slice(message_id.as_bytes());
                bytes
            }
            NotificationBytesType::Reaction {
                channel_id,
                message_id,
            } => {
                let mut bytes = Vec::with_capacity(1 + 16 + 16);
                bytes.push(0x01);
                bytes.extend_from_slice(channel_id.as_bytes());
                bytes.extend_from_slice(message_id.as_bytes());
                bytes
            }
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        if bytes.len() < 1 + 16 + 16 {
            return Err(());
        }
        match bytes[0] {
            0x00 => {
                let channel_id = ChannelId::from_slice(&bytes[1..17]).map_err(|_| ())?;
                let message_id = MessageId::from_slice(&bytes[17..33]).map_err(|_| ())?;
                Ok(NotificationBytesType::Message {
                    channel_id,
                    message_id,
                })
            }
            0x01 => {
                let channel_id = ChannelId::from_slice(&bytes[1..17]).map_err(|_| ())?;
                let message_id = MessageId::from_slice(&bytes[17..33]).map_err(|_| ())?;
                Ok(NotificationBytesType::Reaction {
                    channel_id,
                    message_id,
                })
            }
            _ => Err(()),
        }
    }
}

impl NotificationBytesVersion {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            NotificationBytesVersion::V1 {
                notification_id,
                session_id,
                flags,
                ty,
            } => {
                let mut bytes = Vec::with_capacity(1 + 16 + 16 + 1 + 33);
                bytes.push(0x00);
                bytes.extend_from_slice(notification_id.as_bytes());
                bytes.extend_from_slice(session_id.as_bytes());
                bytes.push(flags.bits());
                bytes.extend_from_slice(&ty.to_bytes());
                bytes
            }
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        if bytes.is_empty() {
            return Err(());
        }

        match bytes[0] {
            0x00 => {
                if bytes.len() < 1 + 16 + 16 + 1 + 33 {
                    return Err(());
                }

                let notification_id = NotificationId::from_slice(&bytes[1..17]).map_err(|_| ())?;
                let session_id = SessionId::from_slice(&bytes[17..33]).map_err(|_| ())?;
                let flags = NotificationBytesFlags::from_bits(bytes[33]).ok_or(())?;
                let ty = NotificationBytesType::from_bytes(&bytes[34..])?;
                Ok(NotificationBytesVersion::V1 {
                    notification_id,
                    session_id,
                    flags,
                    ty,
                })
            }
            _ => Err(()),
        }
    }
}

impl NotificationBytes {
    /// set the session id for this notification payload
    pub fn set_session_id(&mut self, session_id: SessionId) {
        let NotificationBytesVersion::V1 { session_id: s, .. } = &mut self.version;
        *s = session_id;
    }

    /// serialize this notification payload into bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.version.to_bytes()
    }

    /// parse this notification payload from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        let version = NotificationBytesVersion::from_bytes(bytes)?;
        Ok(NotificationBytes { version })
    }
}

impl From<Notification> for NotificationBytes {
    fn from(value: Notification) -> Self {
        let ty = match value.ty {
            NotificationType::Message {
                channel_id,
                message_id,
                ..
            } => NotificationBytesType::Message {
                channel_id,
                message_id,
            },
            NotificationType::Reaction {
                channel_id,
                message_id,
                ..
            } => NotificationBytesType::Reaction {
                channel_id,
                message_id,
            },
        };

        NotificationBytes {
            version: NotificationBytesVersion::V1 {
                notification_id: value.id,
                session_id: SessionId::from(uuid::Uuid::nil()),
                flags: NotificationBytesFlags::empty(),
                ty,
            },
        }
    }
}
