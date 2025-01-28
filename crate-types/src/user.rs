use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::{UserId, UserVerId};
use super::util::deserialize_default_true;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct User {
    pub id: UserId,
    pub version_id: UserVerId,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    // email: Option<String>,
    // avatar: Option<String>,
    #[serde(flatten)]
    pub user_type: UserType,
    pub state: UserState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct UserCreateRequest {
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
