use lamprey_macros::record;

use crate::v1::types::{User, UserId, mirror::seq::UserSeq, presence::Presence};

/// something happened to a user
#[record]
#[serde(tag = "type")]
pub enum DispatchUser {
    UserCreate(UserCreate),
    UserUpdate(UserUpdate),
    UserDelete(UserDelete),
    PresenceUpdate(PresenceUpdate),
}

#[record]
pub struct UserCreate {
    pub user: Box<User>,
    pub seq: UserSeq,
}

#[record]
pub struct UserUpdate {
    pub user: Box<User>,
    pub seq: UserSeq,
}

#[record]
pub struct UserDelete {
    pub user_id: UserId,
    pub seq: UserSeq,
}

#[record]
pub struct PresenceUpdate {
    pub user_id: UserId,
    pub presence: Presence,
}

impl DispatchUser {
    /// id of the user this event happened to
    pub fn user_id(&self) -> UserId {
        match self {
            DispatchUser::UserCreate(a) => a.user.id,
            DispatchUser::UserUpdate(a) => a.user.id,
            DispatchUser::UserDelete(a) => a.user_id,
            DispatchUser::PresenceUpdate(a) => a.user_id,
        }
    }

    /// the user sync sequence number of this event
    pub fn seq(&self) -> Option<UserSeq> {
        match self {
            DispatchUser::UserCreate(a) => Some(a.seq),
            DispatchUser::UserUpdate(a) => Some(a.seq),
            DispatchUser::UserDelete(a) => Some(a.seq),
            DispatchUser::PresenceUpdate(a) => None,
        }
    }
}
