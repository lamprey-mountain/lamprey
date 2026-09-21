use lamprey_macros::record;

use crate::v1::types::{UserId, federation::Hostname, misc::Time};

// TODO: rename types from RoomBan, RoomBanFoo to just Ban, BanFoo

/// represents a restriction on who can join the room
#[record]
pub struct RoomBan {
    /// the user who is banned
    pub user_id: UserId,

    /// the supplied reason why this user should be banned
    pub reason: Option<String>,

    /// when the ban was created
    pub created_at: Time,

    /// when the ban expires
    pub expires_at: Option<Time>,
    // TODO: add type, remove user_id
    // in the future, there will be multiple types of bans. right now there are just user bans.
    // BanId would be changed from UserId to another uuid newtype
    // pub ty: RoomBanType,
}

#[record]
#[serde(tag = "type")]
pub enum RoomBanType {
    /// ban a single user
    User { user_id: UserId },

    /// ban a server by hostname
    Server { hostname: Hostname },

    /// ban an ip cidr range
    Ip {
        // /// the ip address(es) which are banned
        // cidr: IpCidr,
    },
    // TODO: ban emails
    // /// ban email addresses
    // Email { email_pattern: String },
    // TODO: option to require email address
}

#[record]
pub struct RoomBanCreate {
    pub expires_at: Option<Time>,
}

/// create many bans at once
#[record]
pub struct RoomBanBulkCreate {
    /// who to ban
    #[serde(default)]
    #[validate(length(min = 1, max = 256))]
    pub target_ids: Vec<UserId>,

    /// when the ban expires
    pub expires_at: Option<Time>,
}
