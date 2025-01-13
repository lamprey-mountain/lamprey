use std::iter::FromIterator;
use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{Error, Result};

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "permission")]
pub enum Permission {
    Admin,
    RoomManage,
    ThreadCreate,
    ThreadManage,
    ThreadDelete,
    MessageCreate,
    MessageFilesEmbeds,
    MessagePin,
    MessageDelete,
    MessageMassMention,
    MemberKick,
    MemberBan,
    MemberManage,
    InviteCreate,
    InviteManage,
    RoleManage,
    RoleApply,

    View,
    MessageEdit,
}

pub struct Permissions {
    p: HashSet<Permission>,
}

impl Permissions {
    pub fn empty() -> Permissions {
        Permissions {
            p: HashSet::new(),
        }
    }

    #[inline]
    pub fn add(&mut self, perm: Permission) {
        self.p.insert(perm);
    }
    
    #[inline]
    pub fn remove(&mut self, perm: Permission) {
        self.p.remove(&perm);
    }
    
    #[inline]
    pub fn has(&self, perm: Permission) -> bool {
        self.p.contains(&perm)
    }
    
    pub fn ensure(&self, perm: Permission) -> Result<()> {
        if self.has(perm) {
            Ok(())
        } else {
            Err(Error::MissingPermissions)
        }
    }
    
    pub fn ensure_view(&self) -> Result<()> {
        if self.has(Permission::View) {
            Ok(())
        } else {
            Err(Error::NotFound)
        }
    }
}

impl FromIterator<Permission> for Permissions {
    fn from_iter<T: IntoIterator<Item = Permission>>(iter: T) -> Self {
        Permissions { p: iter.into_iter().collect() }
    }
}

impl IntoIterator for Permissions {
    type Item = Permission;
    type IntoIter = std::collections::hash_set::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.p.into_iter()
    }
}
