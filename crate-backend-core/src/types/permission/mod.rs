use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::{ChannelId, Permission, RoomId};

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

// --- NEW API ---

#[derive(Debug, Clone)]
pub enum MemberState {
    Lurker,
    Joined {
        muted: bool,
        deafened: bool,
        timed_out: bool,
        quarantined: bool,
    },
}

pub struct Permissions2<S: CheckState> {
    /// if the user can view this resource
    pub visible: bool,

    /// the kind of resource this permission is for
    pub context: ResourceContext,

    /// set of basic permissions
    pub bits: PermissionBits,

    pub metadata: Permissions2Metadata,

    pub state: S,
}

pub struct Permissions2Metadata {
    pub rank: u16,
    pub member_state: MemberState,
    pub channel_locked: bool,
    pub channel_slowmode_thread_active: bool,
    pub channel_slowmode_message_active: bool,
}

impl ResourceContext {
    fn not_found_error(&self) -> Error {
        let code = match self {
            ResourceContext::Room(_) => ErrorCode::UnknownRoom,
            ResourceContext::Channel(..) | ResourceContext::Thread(..) => ErrorCode::UnknownChannel,
        };
        Error::ApiError(ApiError::from_code(code))
    }
}

#[derive(Debug, Clone)]
pub enum ResourceContext {
    /// trying to do something in a room
    // may be the server room (SERVER_ROOM_ID)
    Room(RoomId),

    /// trying to do something in a channel
    Channel(Option<RoomId>, ChannelId),

    /// trying to do something in a thread channel
    // room, parent, thread id
    Thread(Option<RoomId>, ChannelId, ChannelId),
}

pub struct CheckVisibility;

#[derive(Default)]
pub struct CheckPermissions {
    missing: PermissionBits,
    locked: bool,
    slowmode_thread_bypass_needed: bool,
    slowmode_message_bypass_needed: bool,
}

pub trait CheckState: sealed::Sealed {}
impl CheckState for CheckVisibility {}
impl CheckState for CheckPermissions {}
mod sealed {
    pub trait Sealed {}
    impl Sealed for super::CheckVisibility {}
    impl Sealed for super::CheckPermissions {}
}

impl<T: CheckState> Permissions2<T> {
    pub fn rank(&self) -> u16 {
        self.metadata.rank
    }

    pub fn member_state(&self) -> &MemberState {
        &self.metadata.member_state
    }

    pub fn channel_locked(&self) -> bool {
        self.metadata.channel_locked
    }

    pub fn channel_slowmode_thread_active(&self) -> bool {
        self.metadata.channel_slowmode_thread_active
    }

    pub fn channel_slowmode_message_active(&self) -> bool {
        self.metadata.channel_slowmode_message_active
    }
}

impl Permissions2<CheckVisibility> {
    /// Check if the user has a specific permission.
    /// Admins have all permissions.
    pub fn has(&self, perm: Permission) -> bool {
        self.bits.has(Permission::Admin) || self.bits.has(perm)
    }

    /// Check if the user has any of the given permissions.
    /// Admins have all permissions.
    pub fn has_any(&self, perms: &[Permission]) -> bool {
        self.bits.has(Permission::Admin) || self.bits.has_any(perms)
    }

    /// Check if the user has all of the given permissions.
    /// Admins have all permissions.
    pub fn has_all(&self, perms: &[Permission]) -> bool {
        self.bits.has(Permission::Admin) || self.bits.has_all(perms)
    }

    /// Ensure the user can view this resource, transitioning to CheckPermissions state.
    pub fn ensure_view(self) -> Result<Permissions2<CheckPermissions>> {
        if self.visible {
            Ok(Permissions2 {
                visible: self.visible,
                bits: self.bits,
                context: self.context,
                metadata: self.metadata,
                state: CheckPermissions::default(),
            })
        } else {
            Err(self.context.not_found_error())
        }
    }

    /// Set whether thread slowmode is currently active for this user.
    pub fn with_thread_slowmode_active(mut self, active: bool) -> Self {
        self.metadata.channel_slowmode_thread_active = active;
        self
    }

    /// Set whether message slowmode is currently active for this user.
    pub fn with_message_slowmode_active(mut self, active: bool) -> Self {
        self.metadata.channel_slowmode_message_active = active;
        self
    }
}

impl Permissions2<CheckPermissions> {
    /// Accumulate a required permission
    pub fn needs(&mut self, perm: Permission) -> &mut Self {
        if !self.has(perm) {
            self.state.missing.add(perm);
        }
        self
    }

    pub fn needs_all(&mut self, perms: &[Permission]) -> &mut Self {
        if perms.is_empty() {
            return self;
        }
        let mask = PermissionBits::from_slice(perms);
        let missing = mask & !self.bits;
        self.state.missing |= missing;
        self
    }

    /// the target channel must be unlocked, or the user must be able to bypass it
    pub fn needs_unlocked(&mut self) -> &mut Self {
        if self.metadata.channel_locked && !self.can_bypass_locked() {
            self.state.locked = true;
        }
        self
    }

    /// the channel must not have thread slowmode active, or the user must be able to bypass it
    pub fn needs_slowmode_thread_bypass(&mut self) -> &mut Self {
        if self.metadata.channel_slowmode_thread_active && !self.can_bypass_slowmode() {
            self.state.slowmode_thread_bypass_needed = true;
        }
        self
    }

    /// the channel must not have message slowmode active, or the user must be able to bypass it
    pub fn needs_slowmode_message_bypass(&mut self) -> &mut Self {
        if self.metadata.channel_slowmode_message_active && !self.can_bypass_slowmode() {
            self.state.slowmode_message_bypass_needed = true;
        }
        self
    }

    pub fn check(self) -> Result<Self> {
        if self.state.locked {
            return Err(Error::ApiError(ApiError::from_code(
                ErrorCode::ThreadLocked,
            )));
        }

        if self.state.slowmode_thread_bypass_needed {
            return Err(Error::BadStatic("slowmode in effect"));
        }

        if self.state.slowmode_message_bypass_needed {
            return Err(Error::BadStatic("slowmode in effect"));
        }

        if self.state.missing == PermissionBits::default() {
            Ok(self)
        } else {
            Err(Error::ApiError(ApiError {
                required_permissions: self.state.missing.to_vec(),
                ..ApiError::from_code(ErrorCode::MissingPermissions)
            }))
        }
    }

    pub fn has(&self, perm: Permission) -> bool {
        self.bits.has(Permission::Admin) || self.bits.has(perm)
    }

    pub fn has_any(&self, perms: &[Permission]) -> bool {
        self.bits.has(Permission::Admin) || self.bits.has_any(perms)
    }

    pub fn has_all(&self, perms: &[Permission]) -> bool {
        self.bits.has(Permission::Admin) || self.bits.has_all(perms)
    }

    fn can_bypass_locked(&self) -> bool {
        self.has_any(&[
            Permission::Admin,
            Permission::ChannelManage,
            Permission::ThreadManage,
        ])
    }

    fn can_bypass_slowmode(&self) -> bool {
        self.has_any(&[
            Permission::ChannelManage,
            Permission::ThreadManage,
            Permission::MemberTimeout,
            Permission::ChannelSlowmodeBypass,
        ])
    }
}

impl<T: CheckState> From<Permissions2<T>> for Permissions {
    fn from(perms2: Permissions2<T>) -> Self {
        let mut flags = PermissionsFlags::new();

        match perms2.member_state() {
            MemberState::Lurker => {
                flags.set_cannot_view();
            }
            MemberState::Joined {
                muted,
                deafened,
                timed_out,
                quarantined,
            } => {
                if *muted {
                    flags.set_room_muted();
                }
                if *deafened {
                    flags.set_room_deafened();
                }
                if *timed_out {
                    flags.set_timed_out();
                }
                if *quarantined {
                    flags.set_quarantined();
                }
            }
        }

        if perms2.channel_locked() {
            flags.set_channel_locked();
        }

        let context = match perms2.context {
            ResourceContext::Room(_) => PermissionsContext::Room,
            ResourceContext::Channel(..) | ResourceContext::Thread(..) => {
                PermissionsContext::Channel
            }
        };

        Permissions {
            perms: perms2.bits,
            flags,
            context,
            rank: perms2.rank(),
        }
    }
}
