use common::v1::types::{MessageSync, Permission, oauth::Scope};

use crate::types::permission::PermissionBits;

/// a set of permission checks that must pass
#[derive(Debug, Clone, Default)]
pub struct Requirements<C> {
    needs: PermissionBits,
    assume_visible: bool,
    context: C,
}

// TODO: impl sealed trait for these
#[derive(Debug, Clone, Default)]
pub struct RequirementsRoom;

#[derive(Debug, Clone, Default)]
pub struct RequirementsChannel {
    assume_unlocked: bool,
    slowmode_thread: bool,
    slowmode_message: bool,
}

// pub enum AnyRequirements {
//     Room(...),
//     Channel(...),
// }

mod flex {
    pub trait Seal {}
}

pub trait RequirementsContext: flex::Seal {}

impl<C> Requirements<C> {
    /// user needs this permission
    pub fn permission(&mut self, _perm: Permission) -> &mut Self {
        todo!()
    }

    /// user needs this oauth2 scope
    ///
    /// - if user is being puppeted, check the puppeteer session's scopes.
    /// - servers (federation) are assumed to have all scopes.
    pub fn scope(&mut self, _scope: Scope) -> &mut Self {
        todo!()
    }

    /// assume the user can view this resource, even if permission checks say otherwise
    ///
    /// it used for invites, where the user can see what they're being invited to even though they haven't gained access yet
    pub fn assume_visible(&mut self) -> &mut Self {
        todo!()
    }

    // fn something_suspended to allow suspended users
}

impl Requirements<RequirementsChannel> {
    /// assume the target channel is unlocked
    ///
    /// bypasses lock checks. is used for read-only ops and deletion.
    pub fn assume_unlocked(&mut self) -> &mut Self {
        todo!()
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
        todo!()
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
        todo!()
    }
}

pub fn requirement_from_message_sync(m: &MessageSync) -> Requirements<()> {
    match m {
        _ => todo!(),
    }
}
