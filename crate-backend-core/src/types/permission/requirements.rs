use bitflags::bitflags;
use common::v1::types::Permission;
use common::v1::types::oauth::{Scope, ScopeBits};
use common::v2::types::{ChannelId, RoomId};

use crate::types::permission::PermissionBits;

bitflags! {
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RequirementsFlags: u8 {
        /// the user can always view the target resource
        const AlwaysVisible = 1 << 0;

        /// user must pass slowmode check for thread creation
        const SlowmodeThread = 1 << 1;

        /// user must pass slowmode check for message creation
        const SlowmodeMessage = 1 << 2;

        /// allow suspended users
        const AllowSuspended = 1 << 3;

        /// allow access to locked threads
        const AllowLocked = 1 << 4;

        /// require mfa to be enabled on the user's account
        const RequireMfa = 1 << 5;

        /// require the user to be in sudo mode
        const RequireSudo = 1 << 6;
    }
}

impl RequirementsFlags {
    pub fn always_visible(&self) -> bool {
        self.contains(Self::AlwaysVisible)
    }

    pub fn slowmode_thread(&self) -> bool {
        self.contains(Self::SlowmodeThread)
    }

    pub fn slowmode_message(&self) -> bool {
        self.contains(Self::SlowmodeMessage)
    }

    pub fn allow_suspended(&self) -> bool {
        self.contains(Self::AllowSuspended)
    }

    pub fn allow_locked(&self) -> bool {
        self.contains(Self::AllowLocked)
    }

    pub fn require_mfa(&self) -> bool {
        self.contains(Self::RequireMfa)
    }

    pub fn require_sudo(&self) -> bool {
        self.contains(Self::RequireSudo)
    }
}

/// the kind of resource this permission is for
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequirementsContext {
    /// this is for server-wide permissions
    Server,

    /// this is for room-level permissions
    Room(RoomId),

    /// this is for channel-level permissions, including overwrites
    Channel(ChannelId),
}

/// a set of authorization checks that must pass
#[derive(Debug, Clone)]
pub struct Requirements {
    context: RequirementsContext,
    permissions: PermissionBits,
    scopes: ScopeBits,
    flags: RequirementsFlags,
}

impl Requirements {
    pub fn new(context: RequirementsContext) -> Self {
        Self {
            context,
            permissions: Default::default(),
            scopes: Default::default(),
            flags: Default::default(),
        }
    }

    pub fn new_room(room_id: RoomId) -> Self {
        Self::new(RequirementsContext::Room(room_id))
    }

    pub fn new_channel(channel_id: ChannelId) -> Self {
        Self::new(RequirementsContext::Channel(channel_id))
    }

    pub fn new_server() -> Self {
        Self::new(RequirementsContext::Server)
    }

    /// actor needs this permission
    pub fn permission(&mut self, perm: Permission) -> &mut Self {
        self.permissions.insert(perm.into());
        self
    }

    /// actor needs this oauth2 scope
    ///
    /// - if user is being puppeted, check the puppeteer session's scopes.
    /// - servers (via federation) are assumed to have all scopes.
    pub fn scope(&mut self, scope: Scope) -> &mut Self {
        self.scopes.insert(scope.into());
        self
    }

    /// forcibly allow the user to view this resource
    ///
    /// this is used for invites, where the user can see what they're being invited to even though they haven't gained access yet
    pub fn always_visible(&mut self) -> &mut Self {
        self.flags.insert(RequirementsFlags::AlwaysVisible);
        self
    }

    /// forcibly allow suspended users
    ///
    /// this is used for certain operations
    pub fn allow_suspended(&mut self) -> &mut Self {
        self.flags.insert(RequirementsFlags::AllowSuspended);
        self
    }

    /// forcibly allow access to locked channels
    ///
    /// this is used for read-only operations, as well as deletion.
    pub fn allow_locked(&mut self) -> &mut Self {
        self.flags.insert(RequirementsFlags::AllowLocked);
        self
    }

    /// require mfa to be enabled on the user's account
    pub fn require_mfa(&mut self) -> &mut Self {
        self.flags.insert(RequirementsFlags::RequireMfa);
        self
    }

    /// require the user to be in sudo mode
    pub fn require_sudo(&mut self) -> &mut Self {
        self.flags.insert(RequirementsFlags::RequireSudo);
        self
    }

    /// user must pass message slowmode check (message create ratelimit)
    ///
    /// passes if any of these are true:
    ///
    /// - channel does not have message slowmode active
    /// - slowmode cooldown is not active for this user
    /// - user has `ChannelSlowmodeBypass` or `ChannelManage`
    /// - user has `MemberTimeout` (note: this may be removed soon)
    /// - channel is a thread and user has `ThreadManage`
    pub fn slowmode_message(&mut self) -> &mut Self {
        self.flags.insert(RequirementsFlags::SlowmodeMessage);
        self
    }

    /// user must pass thread slowmode check (thread create ratelimit)
    ///
    /// passes if any of these are true:
    ///
    /// - channel does not have thread slowmode active
    /// - slowmode cooldown is not active for this user
    /// - user has `ChannelSlowmodeBypass` or `ChannelManage`
    /// - user has `MemberTimeout` (note: this may be removed soon)
    /// - channel is a thread and user has `ThreadManage`
    pub fn slowmode_thread(&mut self) -> &mut Self {
        self.flags.insert(RequirementsFlags::SlowmodeThread);
        self
    }

    pub fn get_context(&self) -> RequirementsContext {
        self.context
    }

    pub fn get_permissions(&self) -> PermissionBits {
        self.permissions
    }

    pub fn get_scopes(&self) -> ScopeBits {
        self.scopes
    }

    pub fn get_flags(&self) -> RequirementsFlags {
        self.flags
    }
}

// pub fn requirement_from_message_sync(m: &MessageSync) -> Requirements<()> {
//     match m {
//         _ => todo!(),
//     }
// }
