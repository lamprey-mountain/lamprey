use std::cmp::Ordering;

use crate::prelude::*;
use common::v2::types::UserId;

/// an identifier for a dm channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DmKey(UserId, UserId);

impl DmKey {
    /// create a new dm key, automatically sorting user ids
    ///
    /// fails if a user tries to dm themselves
    // NOTE: should i allow dming yourself?
    pub fn new(a: UserId, b: UserId) -> Result<Self> {
        match a.cmp(&b) {
            Ordering::Less => Ok(Self(a, b)),
            // TODO: make this an api ErrorCode
            Ordering::Equal => Err(Error::BadStatic("cant dm yourself")),
            Ordering::Greater => Ok(Self(b, a)),
        }
    }

    /// get the sorted users
    pub fn get_users(&self) -> (UserId, UserId) {
        (self.0, self.1)
    }
}
