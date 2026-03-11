use std::iter::FromIterator;

use common::v1::types::{
    defaults::ADMIN_ROOM,
    error::{ApiError, ErrorCode},
    Permission,
};

use crate::error::{Error, Result};

pub mod bits;
pub mod flags;

pub use bits::{PermissionBits, BROADCAST_LURKER_PERMS, QUARANTINE_PERMS, VIEW_PERMS};
pub use flags::Permissions2Flags;

/// representation of what permissions a user has
#[derive(Debug, Clone, Default)]
pub struct Permissions2 {
    /// set of basic permissions
    perms: PermissionBits,

    /// special permissions/restrictions
    flags: Permissions2Flags,

    /// the kind of resource this permission is for
    context: Permissions2Context,

    /// the rank of the user in this context
    rank: u16,
}

#[derive(Debug, Clone, Default)]
pub struct Permissions2Builder {
    /// set of basic permissions
    pub perms: PermissionBits,

    /// special permissions/restrictions
    pub flags: Permissions2Flags,

    /// the kind of resource this permission is for
    pub context: Permissions2Context,

    /// the rank of the user in this context
    pub rank: u16,
}

impl Permissions2Builder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn build(self) -> Permissions2 {
        Permissions2 {
            perms: self.perms,
            flags: self.flags,
            context: self.context,
            rank: self.rank,
        }
    }
}

impl Permissions2 {
    /// create a new permissions builder
    #[inline]
    pub fn builder() -> Permissions2Builder {
        Permissions2Builder::new()
    }

    #[inline]
    pub fn has(&self, perm: Permission) -> bool {
        self.perms.has(perm)
    }

    /// ensure that the user is able to view this resource, returning an error if they don't
    #[inline]
    pub fn ensure_view(&self) -> Result<()> {
        if self.flags.can_view() {
            Ok(())
        } else {
            return Err(Error::ApiError(ApiError::from_code(match self.context {
                Permissions2Context::Room => ErrorCode::UnknownRoom,
                Permissions2Context::Channel => ErrorCode::UnknownChannel,
            })));
        }
    }

    /// ensure that the user has a permission, returning an error if they don't
    #[inline]
    pub fn ensure(&self, perm: Permission) -> Result<()> {
        if self.perms.has(perm) {
            Ok(())
        } else {
            if perm == Permission::ViewChannel {
                return self.ensure_view();
            }
            Err(Error::ApiError(ApiError {
                required_permissions: vec![perm],
                ..ApiError::from_code(ErrorCode::MissingPermissions)
            }))
        }
    }

    #[inline]
    pub fn perms(&self) -> PermissionBits {
        self.perms
    }

    #[inline]
    pub fn flags(&self) -> &Permissions2Flags {
        &self.flags
    }

    #[inline]
    pub fn context(&self) -> &Permissions2Context {
        &self.context
    }

    #[inline]
    pub fn rank(&self) -> u16 {
        self.rank
    }

    #[inline]
    pub fn set_context(&mut self, context: Permissions2Context) {
        self.context = context;
    }

    #[inline]
    pub fn set_rank(&mut self, rank: u16) {
        self.rank = rank;
    }

    #[inline]
    pub fn is_channel_locked(&self) -> bool {
        self.flags.is_channel_locked()
    }

    #[inline]
    pub fn can_bypass_locked_channels(&self) -> bool {
        // Users with admin or channel/thread manage can bypass locks
        self.perms.has(Permission::Admin)
            || self.perms.has(Permission::ChannelManage)
            || self.perms.has(Permission::ThreadManage)
    }
}

/// the kind of resource this permission is for
#[derive(Debug, Clone, Default)]
#[repr(u16)]
pub enum Permissions2Context {
    /// this is for room-level permissions
    #[default]
    Room,

    /// this is for channel-level permissions, including overwrites
    Channel,
}

// === old code below ===

impl From<Permissions2> for Permissions {
    #[inline]
    fn from(p2: Permissions2) -> Self {
        let locked_bypass = p2.perms.has(Permission::Admin)
            || p2.perms.has(Permission::ChannelManage)
            || p2.perms.has(Permission::ThreadManage)
            || p2.perms.has(Permission::ThreadLock);

        Permissions {
            p: p2.perms,
            timed_out: p2.flags.is_timed_out(),
            quarantined: p2.flags.is_quarantined(),
            locked_bypass,
            channel_locked: p2.flags.is_channel_locked(),
            lurker: false,
            is_room_member: true,
        }
    }
}

// TODO: remove
/// permission calculator
// this isnt really a permission calculator, more like a representation of what permissions a user has
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Permissions {
    /// the set of permissions this user has
    p: PermissionBits,

    /// whether this user is timed out
    ///
    /// used to determine if they can react with existing reactions or not
    #[serde(default)]
    timed_out: bool,

    /// whether this user is quarantined by automod
    #[serde(default)]
    quarantined: bool,

    /// whether this user can bypass channel/thread locks
    #[serde(default)]
    locked_bypass: bool,

    /// whether this user is trying to access a locked channel/thread
    #[serde(default)]
    channel_locked: bool,

    /// whether the user is lurking a public channel/room (not yet a room member)
    ///
    /// used to determine if they can join voice channels
    #[serde(default)]
    lurker: bool,

    /// whether the user is a member of the room
    #[serde(default)]
    is_room_member: bool,
}

impl Permissions {
    #[inline]
    pub fn empty() -> Permissions {
        Permissions {
            p: PermissionBits::default(),
            timed_out: false,
            quarantined: false,
            locked_bypass: false,
            channel_locked: false,
            lurker: false,
            is_room_member: false,
        }
    }

    #[inline]
    pub fn set_timed_out(&mut self, timed_out: bool) {
        self.timed_out = timed_out;
    }

    #[inline]
    pub fn set_quarantined(&mut self, quarantined: bool) {
        self.quarantined = quarantined;
    }

    #[inline]
    pub fn set_lurker(&mut self, lurker: bool) {
        self.lurker = lurker;
    }

    #[inline]
    pub fn set_is_room_member(&mut self, is_room_member: bool) {
        self.is_room_member = is_room_member;
    }

    #[inline]
    pub fn set_locked_bypass(&mut self, locked_bypass: bool) {
        self.locked_bypass = locked_bypass;
    }

    #[inline]
    pub fn set_channel_locked(&mut self, channel_locked: bool) {
        self.channel_locked = channel_locked;
    }

    #[inline]
    pub fn is_channel_locked(&self) -> bool {
        self.channel_locked
    }

    #[inline]
    pub fn is_member(&self) -> bool {
        self.is_room_member
    }

    pub fn ensure_member(&self) -> Result<()> {
        if self.is_member() {
            Ok(())
        } else {
            Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownRoom)))
        }
    }

    #[inline]
    pub fn add(&mut self, perm: Permission) {
        // Handle implied permissions
        if perm == Permission::Admin {
            let admin_bits = PermissionBits::from_slice(ADMIN_ROOM);
            self.p.add_all(admin_bits);
        } else if perm == Permission::CalendarEventManage {
            self.p.add(Permission::CalendarEventCreate);
        }

        // Add the permission itself
        self.p.add(perm);
    }

    #[inline]
    pub fn remove(&mut self, perm: Permission) {
        // TODO: handle implied permissions?
        self.p.remove(perm);
    }

    /// Add all permissions from a PermissionBits (no implied permission handling)
    #[inline]
    pub fn add_bits(&mut self, bits: PermissionBits) {
        self.p.add_all(bits);
    }

    /// Remove all permissions from a PermissionBits (no implied permission handling)
    #[inline]
    pub fn remove_bits(&mut self, bits: PermissionBits) {
        self.p.remove_all(bits);
    }

    /// Add all permissions from a slice of Permissions (with implied permission handling)
    pub fn add_all(&mut self, perms: &[Permission]) {
        for perm in perms {
            self.add(*perm);
        }
    }

    /// Remove all permissions from a slice of Permissions
    pub fn remove_all(&mut self, perms: &[Permission]) {
        for perm in perms {
            self.remove(*perm);
        }
    }

    #[inline]
    pub fn has(&self, perm: Permission) -> bool {
        if self.timed_out {
            let is_allowed = perm == Permission::ViewChannel || perm == Permission::ViewAuditLog;
            return is_allowed && self.p.has(perm);
        }

        if self.quarantined {
            let is_allowed = perm == Permission::ViewChannel
                || perm == Permission::ViewAuditLog
                || perm == Permission::MemberNickname;
            return is_allowed && self.p.has(perm);
        }

        if self.lurker {
            if !matches!(
                perm,
                Permission::ViewChannel
                    | Permission::ViewAuditLog
                    | Permission::VoiceConnect
                    | Permission::VoiceVad
                    | Permission::VoiceSpeak
            ) {
                return false;
            }
        }

        self.p.has(perm)
    }

    pub fn ensure(&self, perm: Permission) -> Result<()> {
        if self.has(perm) {
            Ok(())
        } else {
            if perm == Permission::ViewChannel {
                return Err(Error::ApiError(ApiError::from_code(
                    ErrorCode::UnknownChannel,
                )));
            }
            Err(Error::ApiError(ApiError {
                required_permissions: vec![perm],
                ..ApiError::from_code(ErrorCode::MissingPermissions)
            }))
        }
    }

    // TODO: use this instead of ensure when checking server permissions
    pub fn ensure_server(&self, perm: Permission) -> Result<()> {
        if self.has(perm) {
            Ok(())
        } else {
            if perm == Permission::ViewChannel {
                return Err(Error::ApiError(ApiError::from_code(
                    ErrorCode::UnknownChannel,
                )));
            }
            Err(Error::ApiError(ApiError {
                required_permissions_server: vec![perm],
                ..ApiError::from_code(ErrorCode::MissingPermissions)
            }))
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

    /// remove all permissions except those in the allowed set
    pub fn mask(&mut self, perms: &[Permission]) {
        let allowed_bits = PermissionBits::from_slice(perms);
        self.p = self.p & allowed_bits;
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

    /// alias for can_use_locked_threads()
    #[inline]
    pub fn can_bypass_locked_channels(&self) -> bool {
        self.can_use_locked_threads()
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
        let mut perms = Permissions::empty();
        for perm in iter {
            perms.add(perm);
        }
        perms
    }
}

impl IntoIterator for Permissions {
    type Item = Permission;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        // Convert PermissionBits to Vec and then into iterator
        let perms: Vec<Permission> = self.p.into();
        perms.into_iter()
    }
}
