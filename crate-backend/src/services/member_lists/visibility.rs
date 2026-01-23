use common::v1::types::{PermissionOverwriteType, RoomMember, UserId};
use uuid::Uuid;

#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct MemberListVisibility {
    /// list of permission overwrites in order from topmost parent to the channel itself
    overwrites: Vec<Vec<VisibilityPermission>>,
}

/// minimal version of PermissionOverwrite that only cares about the `ViewChannel` permission
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct VisibilityPermission {
    /// id of role or user
    pub id: Uuid,

    /// whether this is for a user or role
    pub ty: PermissionOverwriteType,

    /// true if allowed, false if denied
    pub allowed: bool,
}

impl MemberListVisibility {
    // /// check if this member can view a channel with this set of overwrites. has_base is if the member can view all channels by default.
    // // TODO: dedup this code with canonical permission logic
    // pub fn visible_to(&self, user_id: UserId, member: &RoomMember, has_base: bool) -> bool {
    //     let mut has_view = has_base;

    //     // apply each overwrite in order
    //     for ow_set in &self.overwrites {
    //         // apply role allow overwrites
    //         for ow in ow_set {
    //             if ow.ty != PermissionOverwriteType::Role {
    //                 continue;
    //             }

    //             if !member.roles.contains(&ow.id.into()) {
    //                 continue;
    //             }

    //             if ow.allowed {
    //                 has_view = true;
    //             }
    //         }

    //         // apply role deny overwrites
    //         for ow in ow_set {
    //             if ow.ty != PermissionOverwriteType::Role {
    //                 continue;
    //             }

    //             if !member.roles.contains(&ow.id.into()) {
    //                 continue;
    //             }

    //             if !ow.allowed {
    //                 has_view = false;
    //             }
    //         }

    //         // apply user allow overwrites
    //         for ow in ow_set {
    //             if ow.ty != PermissionOverwriteType::User {
    //                 continue;
    //             }

    //             if ow.id != *user_id {
    //                 continue;
    //             }

    //             if ow.allowed {
    //                 has_view = true;
    //             }
    //         }

    //         // apply user deny overwrites
    //         for ow in ow_set {
    //             if ow.ty != PermissionOverwriteType::User {
    //                 continue;
    //             }

    //             if ow.id != *user_id {
    //                 continue;
    //             }

    //             if !ow.allowed {
    //                 has_view = false;
    //             }
    //         }
    //     }

    //     has_view
    // }
}
