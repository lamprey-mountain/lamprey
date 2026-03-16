use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::Permission;

use crate::error::{Error, Result};

pub mod bits;
pub mod flags;

pub use bits::{PermissionBits, BROADCAST_LURKER_PERMS, QUARANTINE_PERMS, VIEW_PERMS};
pub use flags::PermissionsFlags;

/// representation of what permissions a user has
#[derive(Debug, Clone, Default)]
pub struct Permissions {
    /// set of basic permissions
    perms: PermissionBits,

    /// special permissions/restrictions
    flags: PermissionsFlags,

    /// the kind of resource this permission is for
    context: PermissionsContext,

    /// the rank of the user in this context
    rank: u16,
}

#[derive(Debug, Clone, Default)]
pub struct PermissionsBuilder {
    /// set of basic permissions
    pub perms: PermissionBits,

    /// special permissions/restrictions
    pub flags: PermissionsFlags,

    /// the kind of resource this permission is for
    pub context: PermissionsContext,

    /// the rank of the user in this context
    pub rank: u16,
}

/// the kind of resource this permission is for
#[derive(Debug, Clone, Default)]
pub enum PermissionsContext {
    /// this is for room-level permissions
    #[default]
    Room,

    /// this is for channel-level permissions, including overwrites
    Channel,
}

impl PermissionsBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn build(self) -> Permissions {
        Permissions {
            perms: self.perms,
            flags: self.flags,
            context: self.context,
            rank: self.rank,
        }
    }
}

impl Permissions {
    /// create a new permissions builder
    #[inline]
    pub fn builder() -> PermissionsBuilder {
        PermissionsBuilder::new()
    }

    #[inline]
    pub fn has(&self, perm: Permission) -> bool {
        if self.perms.has(Permission::Admin) {
            true
        } else {
            self.perms.has(perm)
        }
    }

    /// check if the user has a permission or if an alternate condition is true
    #[inline]
    pub fn has_or(&self, perm: Permission, alt: bool) -> bool {
        alt || self.has(perm)
    }

    /// ensure that the user is able to view this resource, returning an error if they don't
    ///
    /// If the user cannot view (missing ChannelView permission or explicit cannot_view flag),
    /// returns a 404 error (UnknownRoom/UnknownChannel) to avoid leaking resource existence.
    #[inline]
    pub fn ensure_view(&self) -> Result<()> {
        if self.has(Permission::ChannelView) {
            Ok(())
        } else {
            Err(Error::ApiError(ApiError::from_code(match self.context {
                PermissionsContext::Room => ErrorCode::UnknownRoom,
                PermissionsContext::Channel => ErrorCode::UnknownChannel,
            })))
        }
    }

    /// ensure that the user has a permission, returning an error if they don't
    #[inline]
    pub fn ensure(&self, perm: Permission) -> Result<()> {
        if perm == Permission::ChannelView {
            self.ensure_view()
        } else if self.has(perm) {
            Ok(())
        } else {
            Err(Error::ApiError(ApiError {
                required_permissions: vec![perm],
                ..ApiError::from_code(ErrorCode::MissingPermissions)
            }))
        }
    }

    /// ensure that the user has a permission (server variant with different error message)
    #[inline]
    pub fn ensure_server(&self, perm: Permission) -> Result<()> {
        if perm == Permission::ChannelView {
            self.ensure_view()
        } else if self.has(perm) {
            Ok(())
        } else {
            Err(Error::ApiError(ApiError {
                required_permissions_server: vec![perm],
                ..ApiError::from_code(ErrorCode::MissingPermissions)
            }))
        }
    }

    #[inline]
    pub fn perms(&self) -> PermissionBits {
        self.perms
    }

    #[inline]
    pub fn flags(&self) -> &PermissionsFlags {
        &self.flags
    }

    #[inline]
    pub fn context(&self) -> &PermissionsContext {
        &self.context
    }

    #[inline]
    pub fn rank(&self) -> u16 {
        self.rank
    }

    #[inline]
    pub fn set_context(&mut self, context: PermissionsContext) {
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
        self.has(Permission::Admin)
            || self.has(Permission::ChannelManage)
            || self.has(Permission::ThreadManage)
    }

    /// ensure that the user has all permissions, returning an error if they don't
    ///
    /// Returns a 404 error (UnknownRoom/UnknownChannel) if ViewChannel is missing,
    /// otherwise a 403 error (MissingPermissions). The error payload includes *all*
    /// missing permissions.
    #[inline]
    pub fn ensure_all(&self, perms: &[Permission]) -> Result<()> {
        self.ensure_all_impl(perms, false)
    }

    /// ensure that the user has all permissions (server variant)
    ///
    /// Like `ensure_all`, but uses `required_permissions_server` in the error response.
    #[inline]
    pub fn ensure_all_server(&self, perms: &[Permission]) -> Result<()> {
        self.ensure_all_impl(perms, true)
    }

    fn ensure_all_impl(&self, perms: &[Permission], server: bool) -> Result<()> {
        if perms.is_empty() {
            return Ok(());
        }

        // admins have all permissions
        if self.has(Permission::Admin) {
            return Ok(());
        }

        let required_mask = PermissionBits::from_slice(perms);
        let missing_bits = required_mask & !self.perms;

        // no missing permissions
        if missing_bits == PermissionBits::default() {
            return Ok(());
        }

        // return 404 instead of 403 if a visibility check fails to prevent leaks
        if missing_bits.has(Permission::ChannelView) {
            let code = match self.context {
                PermissionsContext::Room => ErrorCode::UnknownRoom,
                PermissionsContext::Channel => ErrorCode::UnknownChannel,
            };
            return Err(Error::ApiError(ApiError::from_code(code)));
        }

        let missing: Vec<Permission> = missing_bits.to_vec();

        let mut err = ApiError::from_code(ErrorCode::MissingPermissions);
        if server {
            err.required_permissions_server = missing;
        } else {
            err.required_permissions = missing;
        }

        Err(Error::ApiError(err))
    }

    /// whether this user has permissions to bypass slowmode in this channel
    pub fn can_bypass_slowmode(&self) -> bool {
        self.has(Permission::ChannelManage)
            || self.has(Permission::ThreadManage)
            || self.has(Permission::MemberTimeout)
            || self.has(Permission::ChannelSlowmodeBypass)
    }

    /// ensure a channel is either unlocked or that the user has permission to interact with it
    // NOTE: remove? merge ThreadLocked error into ensure_foo()
    pub fn ensure_unlocked(&self) -> Result<()> {
        if !self.is_channel_locked() {
            return Ok(());
        }

        if !self.can_bypass_locked_channels() {
            return Err(Error::ApiError(ApiError::from_code(
                ErrorCode::ThreadLocked,
            )));
        }

        Ok(())
    }
}

impl IntoIterator for Permissions {
    type Item = Permission;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let perms: Vec<Permission> = self.perms.into();
        perms.into_iter()
    }
}
