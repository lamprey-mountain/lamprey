use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// an oauth scope
///
/// WORK IN PROGRESS!!! SUBJECT TO CHANGE!!!
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// basic user profle information
    ///
    /// affects user_get and oauth_userinfo
    #[serde(alias = "openid")]
    Identify,

    /// access email addresses
    ///
    /// affects user_get and oauth_userinfo
    Email,

    /// read access to all data, except those with specific scopes
    Read,

    /// write access to all data, except those with specific scopes
    ///
    /// intended for custom clients
    Write,

    /// access to moderation endpoints
    ///
    /// includes these endpoints:
    ///
    /// - room_member_update, room_member_delete, thread_member_delete
    /// - room_ban_create, room_ban_create_bulk, room_ban_remove
    /// - message_moderate, message_delete, message_version_delete (for messages that aren't yours/would require moderator perms for)
    /// - thread_update (if this would require moderation permissions, you can edit your own thread title/topic)
    /// - thread_remove, thread_restore, thread_lock, thread_unlock
    /// - thread_list_removed (nb. this is the only GET endpoint here)
    /// - voice_state_disconnect, voice_state_move
    /// - invite_delete
    /// - role_create, role_update, role_delete, role_reorder
    /// - role_member_add, role_member_remove, role_member_bulk_edit
    /// - emoji_create, emoji_delete
    /// - reaction_purge
    Moderate,

    /// access to user relationships
    ///
    /// includes these endpoints:
    ///
    /// - friend_list, block_list
    #[serde(rename = "relations.read")]
    RelationsRead,

    /// access to user relationships
    ///
    /// includes these endpoints:
    ///
    /// - friend_add, friend_remove, block_add, block_remove
    #[serde(rename = "relations.write")]
    RelationsWrite,

    /// Read/write access to /auth
    ///
    /// - an extremely dangerous scope to grant!
    /// - affects everything under /auth
    Auth,
}
