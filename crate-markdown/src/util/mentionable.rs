// TODO: move to common

use core::fmt;

use lamprey_common::{
    v1::types::{Channel, Role, User},
    v2::types::{ChannelId, RoleId, UserId},
};

use crate::ast::inline::MentionData;

/// something that can be mentioned
pub trait Mentionable {
    fn mention(&self) -> Format;
}

// TODO: handle custom emoji
pub struct Format(MentionData);

impl Mentionable for UserId {
    fn mention(&self) -> Format {
        Format(MentionData::User(*self))
    }
}

impl Mentionable for User {
    fn mention(&self) -> Format {
        Format(MentionData::User(self.id))
    }
}

impl Mentionable for ChannelId {
    fn mention(&self) -> Format {
        Format(MentionData::Channel(*self))
    }
}

impl Mentionable for Channel {
    fn mention(&self) -> Format {
        Format(MentionData::Channel(self.id))
    }
}

impl Mentionable for RoleId {
    fn mention(&self) -> Format {
        Format(MentionData::Role(*self))
    }
}

impl Mentionable for Role {
    fn mention(&self) -> Format {
        Format(MentionData::Role(self.id))
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            MentionData::User(id) => write!(f, "<@{id}>"),
            MentionData::Role(id) => write!(f, "<&{id}>"),
            MentionData::Channel(id) => write!(f, "<#{id}>"),
            MentionData::Everyone => write!(f, "@everyone"),
        }
    }
}
