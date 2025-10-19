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
