//! notification binary encoding

use crate::v1::types::{
    notifications::{Notification, NotificationType},
    ChannelId, MessageId, NotificationId, SessionId, UserId,
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
        ty: NotificationBytesType,
    },
}

#[derive(Debug, Clone)]
pub enum NotificationBytesType {
    // 0x00
    Message {
        channel_id: ChannelId,
        message_id: MessageId,
        flags: NotificationBytesMessagesFlags,
    },

    // 0x01
    Reaction {
        channel_id: ChannelId,
        message_id: MessageId,
        // TODO: add reaction_key
    },

    // 0x02
    FriendRequestSent {
        user_id: UserId,
    },

    // 0x03
    FriendRequestReceived {
        user_id: UserId,
    },

    // 0x04
    FriendRequestAccepted {
        user_id: UserId,
    },

    // 0x05
    Thread {
        thread_id: ChannelId,
    },
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NotificationBytesMessagesFlags: u8 {
        /// this notification was triggered by an @user mention
        const MENTION_USER = 1 << 0;

        /// this notification was triggered by an @everyone or @here mention
        const MENTION_EVERYONE = 1 << 1;

        /// this notification was triggered by a @role mention
        const MENTION_ROLE= 1 << 2;

        /// this notification was triggered by a reply
        const REPLY = 1 << 3;
    }
}

impl NotificationBytesType {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            NotificationBytesType::Message {
                channel_id,
                message_id,
                flags,
            } => {
                let mut bytes = Vec::with_capacity(1 + 16 + 16 + 1);
                bytes.push(0x00);
                bytes.extend_from_slice(channel_id.as_bytes());
                bytes.extend_from_slice(message_id.as_bytes());
                bytes.push(flags.bits());
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
            NotificationBytesType::FriendRequestSent { user_id } => {
                let mut bytes = Vec::with_capacity(1 + 16);
                bytes.push(0x02);
                bytes.extend_from_slice(user_id.as_bytes());
                bytes
            }
            NotificationBytesType::FriendRequestReceived { user_id } => {
                let mut bytes = Vec::with_capacity(1 + 16);
                bytes.push(0x03);
                bytes.extend_from_slice(user_id.as_bytes());
                bytes
            }
            NotificationBytesType::FriendRequestAccepted { user_id } => {
                let mut bytes = Vec::with_capacity(1 + 16);
                bytes.push(0x04);
                bytes.extend_from_slice(user_id.as_bytes());
                bytes
            }
            NotificationBytesType::Thread { thread_id } => {
                let mut bytes = Vec::with_capacity(1 + 16);
                bytes.push(0x05);
                bytes.extend_from_slice(thread_id.as_bytes());
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
                if bytes.len() < 1 + 16 + 16 + 1 {
                    return Err(());
                }
                let channel_id = ChannelId::from_slice(&bytes[1..17]).map_err(|_| ())?;
                let message_id = MessageId::from_slice(&bytes[17..33]).map_err(|_| ())?;
                let flags = NotificationBytesMessagesFlags::from_bits(bytes[33]).ok_or(())?;
                Ok(NotificationBytesType::Message {
                    channel_id,
                    message_id,
                    flags,
                })
            }
            0x01 => {
                if bytes.len() < 1 + 16 + 16 {
                    return Err(());
                }
                let channel_id = ChannelId::from_slice(&bytes[1..17]).map_err(|_| ())?;
                let message_id = MessageId::from_slice(&bytes[17..33]).map_err(|_| ())?;
                Ok(NotificationBytesType::Reaction {
                    channel_id,
                    message_id,
                })
            }
            0x02 => {
                if bytes.len() < 1 + 16 {
                    return Err(());
                }
                let user_id = UserId::from_slice(&bytes[1..17]).map_err(|_| ())?;
                Ok(NotificationBytesType::FriendRequestSent { user_id })
            }
            0x03 => {
                if bytes.len() < 1 + 16 {
                    return Err(());
                }
                let user_id = UserId::from_slice(&bytes[1..17]).map_err(|_| ())?;
                Ok(NotificationBytesType::FriendRequestReceived { user_id })
            }
            0x04 => {
                if bytes.len() < 1 + 16 {
                    return Err(());
                }
                let user_id = UserId::from_slice(&bytes[1..17]).map_err(|_| ())?;
                Ok(NotificationBytesType::FriendRequestAccepted { user_id })
            }
            0x05 => {
                if bytes.len() < 1 + 16 {
                    return Err(());
                }
                let thread_id = ChannelId::from_slice(&bytes[1..17]).map_err(|_| ())?;
                Ok(NotificationBytesType::Thread { thread_id })
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
                ty,
            } => {
                let ty_bytes = ty.to_bytes();
                let mut bytes = Vec::with_capacity(1 + 16 + 16 + ty_bytes.len());
                bytes.push(0x00);
                bytes.extend_from_slice(notification_id.as_bytes());
                bytes.extend_from_slice(session_id.as_bytes());
                bytes.extend_from_slice(&ty_bytes);
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
                if bytes.len() < 1 + 16 + 16 {
                    return Err(());
                }

                let notification_id = NotificationId::from_slice(&bytes[1..17]).map_err(|_| ())?;
                let session_id = SessionId::from_slice(&bytes[17..33]).map_err(|_| ())?;
                let ty = NotificationBytesType::from_bytes(&bytes[33..])?;
                Ok(NotificationBytesVersion::V1 {
                    notification_id,
                    session_id,
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
                mention_everyone,
                mention_role,
                reply,
                ..
            } => {
                let mut flags = NotificationBytesMessagesFlags::empty();
                if mention_everyone {
                    flags |= NotificationBytesMessagesFlags::MENTION_EVERYONE;
                }
                if mention_role {
                    flags |= NotificationBytesMessagesFlags::MENTION_ROLE;
                }
                if reply {
                    flags |= NotificationBytesMessagesFlags::REPLY;
                }
                NotificationBytesType::Message {
                    channel_id,
                    message_id,
                    flags,
                }
            }
            NotificationType::Reaction {
                channel_id,
                message_id,
                ..
            } => NotificationBytesType::Reaction {
                channel_id,
                message_id,
            },
            NotificationType::FriendRequestSent { user_id } => {
                NotificationBytesType::FriendRequestSent { user_id }
            }
            NotificationType::FriendRequestReceived { user_id } => {
                NotificationBytesType::FriendRequestReceived { user_id }
            }
            NotificationType::FriendRequestAccepted { user_id } => {
                NotificationBytesType::FriendRequestAccepted { user_id }
            }
            NotificationType::Thread { thread_id, .. } => {
                NotificationBytesType::Thread { thread_id }
            }
        };

        NotificationBytes {
            version: NotificationBytesVersion::V1 {
                notification_id: value.id,
                session_id: SessionId::from(uuid::Uuid::nil()),
                ty,
            },
        }
    }
}
