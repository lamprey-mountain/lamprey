use std::collections::HashSet;
use std::iter::FromIterator;

use types::{Permission, ADMIN_ROOM};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Permissions {
    p: HashSet<Permission>,
}

impl Permissions {
    pub fn empty() -> Permissions {
        Permissions { p: HashSet::new() }
    }

    #[inline]
    pub fn add(&mut self, perm: Permission) {
        if perm == Permission::Admin {
            self.p.extend(ADMIN_ROOM);
        }
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
        let mut p = HashSet::new();
        for i in iter {
            if i == Permission::Admin {
                p.extend(ADMIN_ROOM);
            }
            p.insert(i);
        }
        Permissions { p }
    }
}

impl IntoIterator for Permissions {
    type Item = Permission;
    type IntoIter = std::collections::hash_set::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.p.into_iter()
    }
}
