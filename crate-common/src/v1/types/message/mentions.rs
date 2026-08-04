use lamprey_macros::record;

use crate::v1::types::{ChannelId, ChannelType, EmojiId, RoleId, RoomId, UserId};

/// who/what this message notified on send
#[record]
#[derive(Default)]
pub struct Mentions {
    // TODO: add validate attrs
    /// the users that were mentioned
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(min_length = 0, max_length = 128)]
    pub users: Vec<MentionsUser>,

    /// the roles that were mentioned
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(min_length = 0, max_length = 128)]
    pub roles: Vec<MentionsRole>,

    /// the channels that were mentioned
    // TODO: populate these channels. this is useful for
    // - bots that don't sync everything
    // - for threads which are lazy loaded
    // - forwards, for consistent rendering. channel types/names will be "leaked" since if someone forwards a message they probably intend it to be rendered the same way they saw it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(min_length = 0, max_length = 128)]
    pub channels: Vec<MentionsChannel>,

    /// the custom emojis that were used in this message
    // TODO: enforce no more than 128 unique custom emoji per message
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(min_length = 0, max_length = 128)]
    pub emojis: Vec<MentionsEmoji>,

    /// if this message mentions everyone
    #[serde(default)]
    pub everyone: bool,
}

/// a mentioned user
#[record]
pub struct MentionsUser {
    /// the id of this user
    pub id: UserId,

    /// the resolved name (either the room member nickname or the user's name)
    pub resolved_name: String,
}

/// a mentioned role
#[record]
pub struct MentionsRole {
    /// the id of this role
    pub id: RoleId,
    // // TODO: add this
    // /// the name of this role
    // pub name: String,
}

/// a mentioned channel
#[record]
pub struct MentionsChannel {
    /// the id of this channel
    pub id: ChannelId,

    /// the room this is in
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    /// the type of this channel
    #[serde(rename = "type")]
    pub ty: ChannelType,

    /// the name of this channel
    pub name: String,
}

/// a custom emoji used in the message
#[record]
pub struct MentionsEmoji {
    /// the id of this emoji
    pub id: EmojiId,

    /// the name of this emoji
    pub name: String,

    /// if this emoji is animated
    pub animated: bool,
}

impl Mentions {
    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
            && self.roles.is_empty()
            && self.channels.is_empty()
            && self.emojis.is_empty()
            && !self.everyone
    }
}
