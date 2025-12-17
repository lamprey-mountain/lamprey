/// the maximum number of roles per room. clients should be able to fetch everything in one request.
pub const MAX_ROLE_COUNT: u32 = 1024;

/// the maximum number of active channels per room. clients should be able to fetch everything in one request.
pub const MAX_CHANNEL_COUNT: u32 = 1024;

/// the maximum number of permission overwrites per channel
pub const MAX_PERMISSION_OVERWRITES: u32 = 64;

/// the maximum number of unique reaction emoji per message
pub const MAX_UNIQUE_REACTIONS: u32 = 20;

/// the maximum number of custom emoji per room. clients should be able to fetch everything in one request.
pub const MAX_CUSTOM_EMOJI: u32 = 1024;

/// the maximum number of pinned messages per channel. clients should be able to fetch everything in one request.
pub const MAX_PINNED_MESSAGES: u32 = 1024;

/// the maximum number of role members to add to a thread when a role is mentioned.
pub const MAX_ROLE_MENTION_MEMBERS_ADD: u32 = 50;

/// the maximum number of members to allow in group dm.
pub const MAX_GDM_MEMBERS: u32 = 16;

/// the maximum number of webhooks per channel
pub const MAX_CHANNEL_WEBHOOKS: u32 = 16;

/// the maximum number of rooms a user can be in.
pub const MAX_ROOM_JOINS: u32 = 128;

/// how many days to retain audit log entries
pub const RETENTION_AUDIT_LOG: u32 = 90;

/// how many days to retain room analytics entries
pub const RETENTION_ROOM_ANALYTICS: u32 = 180;
