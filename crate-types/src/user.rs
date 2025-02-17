use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::util::Diff;
use crate::MediaId;

use super::util::deserialize_default_true;
use super::{UserId, UserVerId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct User {
    pub id: UserId,
    pub version_id: UserVerId,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    // NOTE: do i want to resolve media here?
    pub avatar: Option<MediaId>,
    // email: Option<String>,
    #[serde(flatten)]
    pub user_type: UserType,
    pub state: UserState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct UserCreate {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,

    #[serde(deserialize_with = "deserialize_default_true")]
    pub is_bot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct UserPatch {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub status: Option<Option<String>>,
    pub avatar: Option<Option<MediaId>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum UserType {
    /// a normal user
    Default,

    /// makes two users be considered the same user
    Alias { alias_id: UserId },

    /// automated account
    Bot { owner_id: UserId },

    /// system/service account
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum UserState {
    Active,
    Suspended,
    Deleted,
}

// impl User {
//     pub fn can_view(&self, other: &User) -> bool {
//         match other.user_type {
//             UserType::Default => false,
//             UserType::Alias { alias_id } => self.id == alias_id,
//             UserType::Bot { owner_id } => self.id == owner_id,
//             UserType::System => true,
//         }
//     }
// }

impl Diff<User> for UserPatch {
    fn changes(&self, other: &User) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.avatar.changes(&other.avatar)
            || self.status.changes(&other.status)
    }
}
