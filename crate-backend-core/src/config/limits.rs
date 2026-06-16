use common::v1::types::redex::EvalLimits;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    /// limits for the server
    #[serde(default)]
    pub server: LimitsServer,

    /// default limits for rooms
    #[serde(default)]
    pub room: LimitsRoom,

    /// default limits for users
    #[serde(default)]
    pub user: LimitsUser,

    #[serde(default = "EvalLimits::strict")]
    pub scripts: EvalLimits,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            server: LimitsServer::default(),
            room: LimitsRoom::default(),
            user: LimitsUser::default(),
            scripts: EvalLimits::strict(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsServer {
    /// the maximum incoming http request size
    #[serde(default = "default_max_request_size")]
    pub max_request_size: usize,
}

fn default_max_request_size() -> usize {
    1024 * 1024 * 16
}

impl Default for LimitsServer {
    fn default() -> Self {
        Self {
            max_request_size: default_max_request_size(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsRoom {
    /// the maximum number of roles that can be created in a room
    #[serde(default = "default_max_roles")]
    pub max_roles: u16,

    /// the maximum number of channels that can be created in a room
    #[serde(default = "default_max_channels")]
    pub max_channels: u16,

    /// the maximum number of permission overwrites per channel
    #[serde(default = "default_max_permission_overwrites")]
    pub max_permission_overwrites: u32,

    /// the maximum number of unique reaction emoji per message
    #[serde(default = "default_max_unique_reactions")]
    pub max_unique_reactions: u32,

    /// the maximum number of pinned messages per channel. clients should be able to fetch everything in one request.
    #[serde(default = "default_max_pinned_messages")]
    pub max_pinned_messages: u32,

    /// the maximum number of role members to add to a thread when a role is mentioned.
    // TODO: rename to something more clear?
    #[serde(default = "default_max_role_mention_members_add")]
    pub max_role_mention_members_add: u32,

    /// the maximum number of webhooks per channel
    #[serde(default = "default_max_channel_webhooks")]
    pub max_channel_webhooks: u32,

    /// the maximum number of webhooks per room
    #[serde(default = "default_max_total_webhooks")]
    pub max_total_webhooks: u32,
}

fn default_max_roles() -> u16 {
    1024
}
fn default_max_channels() -> u16 {
    1024
}
fn default_max_permission_overwrites() -> u32 {
    64
}
fn default_max_unique_reactions() -> u32 {
    20
}
fn default_max_pinned_messages() -> u32 {
    1024
}
fn default_max_role_mention_members_add() -> u32 {
    50
}
fn default_max_channel_webhooks() -> u32 {
    16
}
fn default_max_total_webhooks() -> u32 {
    1024
}

impl Default for LimitsRoom {
    fn default() -> Self {
        Self {
            max_roles: default_max_roles(),
            max_channels: default_max_channels(),
            max_permission_overwrites: default_max_permission_overwrites(),
            max_unique_reactions: default_max_unique_reactions(),
            max_pinned_messages: default_max_pinned_messages(),
            max_role_mention_members_add: default_max_role_mention_members_add(),
            max_channel_webhooks: default_max_channel_webhooks(),
            max_total_webhooks: default_max_total_webhooks(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsUser {
    /// the maximum number of emails a user can have
    #[serde(default = "default_max_emails")]
    pub max_emails: u8,

    /// the maximum number of rooms a user can be in.
    #[serde(default = "default_max_room_joins")]
    pub max_room_joins: u32,

    /// the maximum number of public connections a user can have
    ///
    /// ie. connections that are not `ConnectionVisibility::Private`
    #[serde(default = "default_max_public_connections")]
    pub max_public_connections: u8,
}

fn default_max_emails() -> u8 {
    10
}
fn default_max_room_joins() -> u32 {
    128
}
fn default_max_public_connections() -> u8 {
    32
}

impl Default for LimitsUser {
    fn default() -> Self {
        Self {
            max_emails: default_max_emails(),
            max_room_joins: default_max_room_joins(),
            max_public_connections: default_max_public_connections(),
        }
    }
}
