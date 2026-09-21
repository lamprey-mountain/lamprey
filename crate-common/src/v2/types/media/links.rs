use crate::v1::types::{
    ChannelId, EmbedId, MessageId, MessageVerId, RedexId, RedexVerId, RoomId, UserId,
};
use lamprey_macros::record;

/// describes how this piece of media is linked to another resource
///
/// objects can be linked to multiple objects; for example, media linked to
/// `Message`s also have links to each `MessageVersion` they're referenced in.
// TODO(?): rename to MediaLink
#[record]
#[derive(PartialEq, Eq, Hash)]
#[serde(tag = "type")]
pub enum MediaLinkType {
    /// this piece of media is linked to a message
    ///
    /// this link should never exist on its own, and always come with at *least
    /// one* MessageVersion link and *at most one* Embed link
    Message {
        channel_id: ChannelId,
        message_id: MessageId,
    },

    /// this piece of media is linked to a message version
    ///
    /// this link should never exist on its own, and always come with *exactly
    /// one* Message link and *at most one* Embed link
    MessageVersion {
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
    },

    /// this piece of media is used as a user avatar
    UserAvatar { user_id: UserId },

    /// this piece of media is used as a user banner
    UserBanner { user_id: UserId },

    /// this piece of media is used as a channel icon
    ChannelIcon { channel_id: ChannelId },

    /// this piece of media is used as a room icon
    RoomIcon { room_id: RoomId },

    /// this piece of media is used as a room banner
    RoomBanner { room_id: RoomId },

    /// this piece of media is embedded in a message
    // NOTE: auth checks copy Message
    // NOTE: should never exist on its own, always comes with a Message + MessageVersion link
    Embed {
        // TODO: rename to embed_id
        // TODO: add channel_id, message_id
        id: EmbedId,
    },

    /// this piece of media is used as a custom emoji
    CustomEmoji {
        room_id: RoomId,
        // TODO: add emoji_id: EmojiId,
    },

    /// this piece of media is a script
    ///
    /// this link should never exist on its own, and always come with at *least
    /// one* ScriptVersion link
    // TODO: rename to Redex
    Script {
        channel_id: ChannelId,
        script_id: RedexId,
    },

    /// this piece of media is a script version
    ///
    /// this link should never exist on its own, and always come with *exactly
    /// one* Script link
    // TODO: rename to RedexVersion
    ScriptVersion {
        channel_id: ChannelId,
        script_id: RedexId,
        version_id: RedexVerId,
    },

    /// this piece of media is used in a document
    // TODO: more granular linking
    Document {
        channel_id: ChannelId,
        document_id: ChannelId,
    },
}

impl MediaLinkType {
    #[inline]
    pub fn message(channel_id: ChannelId, message_id: MessageId) -> Self {
        Self::Message {
            channel_id,
            message_id,
        }
    }

    #[inline]
    pub fn message_version(
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
    ) -> Self {
        Self::MessageVersion {
            channel_id,
            message_id,
            version_id,
        }
    }

    // TODO
}
