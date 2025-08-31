// nb. private can be done with thread permission overwrites, but that means
// that if an ordinary user wants to create a thread they need to be able to
// edit permissions. this is doable, but could lead to some finnicky permissions

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use serde::{Deserialize, Serialize};

/// who can view this room or thread
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Visibility {
    /// nobody can read except members
    Private,

    /// anyone can read, not indexed
    Unlisted,

    /// anyone can read, also is indexed
    Public,
}

/// who can join this room
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Joinability {
    /// an invite is required
    Invite,

    /// anyone can join
    Public,
}

/// stricter visibility takes precedence over weaker visibility
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Access {
    /// who can see this room/thread
    pub visibility: Visibility,

    /// who can join this room
    pub joinability: Joinability,
}

impl Visibility {
    pub fn inherit_from(&self, parent: &Visibility) -> Visibility {
        *self.max(parent)
    }
}

impl Joinability {
    pub fn inherit_from(&self, parent: &Joinability) -> Joinability {
        *self.max(parent)
    }
}

impl Access {
    pub fn inherit_from(&self, parent: &Access) -> Access {
        Access {
            visibility: self.visibility.inherit_from(&parent.visibility),
            joinability: self.joinability.inherit_from(&parent.joinability),
        }
    }
}
