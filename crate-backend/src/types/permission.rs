use std::collections::HashSet;
use std::iter::FromIterator;

use common::v1::types::{defaults::ADMIN_ROOM, Permission};

use crate::error::{Error, Result};

/// permission calculator
#[derive(Debug, Clone)]
pub struct Permissions {
    /// the set of permissions this user has
    p: HashSet<Permission>,

    /// whether this user is timed out
    timed_out: bool,

    /// whether this user is quarantined by automod
    quarantined: bool,

    /// whether this user can bypass channel/thread locks
    locked_bypass: bool,

    /// whether this user is trying to access a locked channel/thread
    channel_locked: bool,

    /// whether this user is a lurker (not part of the room yet)
    lurker: bool,
}

impl Permissions {
    pub fn empty() -> Permissions {
        Permissions {
            p: HashSet::new(),
            timed_out: false,
            quarantined: false,
            locked_bypass: false,
            channel_locked: false,
            lurker: false,
        }
    }

    pub fn set_timed_out(&mut self, timed_out: bool) {
        self.timed_out = timed_out;
    }

    pub fn set_quarantined(&mut self, quarantined: bool) {
        self.quarantined = quarantined;
    }

    pub fn set_lurker(&mut self, lurker: bool) {
        self.lurker = lurker;
    }

    pub fn set_locked_bypass(&mut self, locked_bypass: bool) {
        self.locked_bypass = locked_bypass;
    }

    pub fn set_channel_locked(&mut self, channel_locked: bool) {
        self.channel_locked = channel_locked;
    }

    pub fn is_channel_locked(&self) -> bool {
        self.channel_locked
    }

    #[inline]
    pub fn add(&mut self, perm: Permission) {
        if perm == Permission::Admin {
            self.p.extend(ADMIN_ROOM);
        }

        if perm == Permission::CalendarEventManage {
            self.p.insert(Permission::CalendarEventCreate);
        }

        // TODO: more implied permissions?
        self.p.insert(perm);
    }

    #[inline]
    pub fn remove(&mut self, perm: Permission) {
        // TODO: handle implied permissions?
        self.p.remove(&perm);
    }

    #[inline]
    pub fn has(&self, perm: Permission) -> bool {
        if self.timed_out {
            return perm == Permission::ViewChannel || perm == Permission::ViewAuditLog;
        }

        if self.quarantined {
            return perm == Permission::ViewChannel
                || perm == Permission::ViewAuditLog
                || perm == Permission::MemberNickname;
        }

        if self.lurker {
            return perm == Permission::ViewChannel || perm == Permission::ViewAuditLog;
            // FIXME: these three should be enabled in Broadcast channels
            // || perm == Permission::VoiceConnect
            // || perm == Permission::VoiceVad
            // || perm == Permission::VoiceSpeak
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

    /// whether this user has permissions to bypass slowmode in this channel
    pub fn can_bypass_slowmode(&self) -> bool {
        self.has(Permission::ChannelManage)
            || self.has(Permission::ThreadManage)
            || self.has(Permission::MemberTimeout)
            || self.has(Permission::BypassSlowmode)
    }

    /// whether this user has permissions to bypass this channel's lock (if it exists)
    pub fn can_use_locked_threads(&self) -> bool {
        self.locked_bypass
            || self.has(Permission::ThreadManage)
            || self.has(Permission::ChannelManage)
            || self.has(Permission::ThreadLock)
    }

    /// ensure a channel is either unlocked or that the user has permission to interact with it
    pub fn ensure_unlocked(&self) -> Result<()> {
        if !self.is_channel_locked() {
            return Ok(());
        }

        if !self.can_use_locked_threads() {
            return Err(Error::BadStatic("thread is locked"));
        }

        Ok(())
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
            quarantined: false,
            locked_bypass: false,
            channel_locked: false,
            lurker: false,
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
