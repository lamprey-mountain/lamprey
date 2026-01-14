use std::collections::HashSet;
use std::iter::FromIterator;

use common::v1::types::{defaults::ADMIN_ROOM, Permission};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Permissions {
    p: HashSet<Permission>,
    timed_out: bool,
}

impl Permissions {
    pub fn empty() -> Permissions {
        Permissions {
            p: HashSet::new(),
            timed_out: false,
        }
    }

    pub fn set_timed_out(&mut self, timed_out: bool) {
        self.timed_out = timed_out;
    }

    #[inline]
    pub fn add(&mut self, perm: Permission) {
        if perm == Permission::Admin {
            self.p.extend(ADMIN_ROOM);
        }
        if perm == Permission::CalendarEventManage {
            self.p.insert(Permission::CalendarEventCreate);
        }
        self.p.insert(perm);
    }

    #[inline]
    pub fn remove(&mut self, perm: Permission) {
        self.p.remove(&perm);
    }

    #[inline]
    pub fn has(&self, perm: Permission) -> bool {
        if self.timed_out {
            return perm == Permission::ViewChannel || perm == Permission::ViewAuditLog;
        }
        self.p.contains(&perm)
    }

    pub fn ensure(&self, perm: Permission) -> Result<()> {
        if self.has(perm) {
            Ok(())
        } else {
            if perm == Permission::ViewChannel {
                return Err(Error::NotFound);
            }
            Err(Error::MissingPermissions)
        }
    }

    // PERF: optimize checking
    // TODO: better error messages - return all permissions that are required instead of only the first one
    pub fn ensure_all(&self, perms: &[Permission]) -> Result<()> {
        for perm in perms {
            self.ensure(*perm)?;
        }

        Ok(())
    }

    pub fn mask(&mut self, perms: &[Permission]) {
        let mut new = HashSet::new();
        for p in perms {
            if self.has(*p) {
                new.insert(*p);
            }
        }
        self.p = new;
    }

    pub fn can_bypass_slowmode(&self) -> bool {
        self.has(Permission::ChannelManage)
            || self.has(Permission::ThreadManage)
            || self.has(Permission::MemberTimeout)
            || self.has(Permission::BypassSlowmode)
    }

    pub fn can_use_locked_threads(&self) -> bool {
        self.has(Permission::ThreadManage)
            || self.has(Permission::ChannelManage)
            || self.has(Permission::ThreadLock)
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
        Permissions {
            p,
            timed_out: false,
        }
    }
}

impl IntoIterator for Permissions {
    type Item = Permission;
    type IntoIter = std::collections::hash_set::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.p.into_iter()
    }
}
