use common::v1::types::{
    Permission, PermissionOverwrite, PermissionOverwriteType, RoomId, RoomMember, UserId,
};
use std::collections::HashSet;
use uuid::Uuid;

/// minimal calculated permissions for who can see this list
#[derive(Debug, Default, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemberListVisibility {
    /// flat list of minimal permission overwrites in application order
    overwrites: Vec<VisibilityPermission>,
}

/// minimal version of PermissionOverwrite that only cares about the `ViewChannel` permission
#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VisibilityPermission {
    /// id of role or user
    pub id: Uuid,

    /// whether this is for a user or role
    pub ty: PermissionOverwriteType,

    /// true if allowed, false if denied
    pub allowed: bool,
}

impl MemberListVisibility {
    /// create a minimal visibility set from full overwrites
    pub fn from_overwrites(room_id: RoomId, levels: Vec<Vec<PermissionOverwrite>>) -> Self {
        let mut sequence = Vec::new();
        for ow_set in levels {
            // application order within each level:
            // 1. everyone allow
            for ow in &ow_set {
                if ow.id != *room_id {
                    continue;
                }
                if ow.allow.contains(&Permission::ViewChannel) {
                    sequence.push(VisibilityPermission {
                        id: ow.id,
                        ty: ow.ty,
                        allowed: true,
                    });
                }
            }
            // 2. everyone deny
            for ow in &ow_set {
                if ow.id != *room_id {
                    continue;
                }
                if ow.deny.contains(&Permission::ViewChannel) {
                    sequence.push(VisibilityPermission {
                        id: ow.id,
                        ty: ow.ty,
                        allowed: false,
                    });
                }
            }
            // 3. role allow
            for ow in &ow_set {
                if ow.ty != PermissionOverwriteType::Role || ow.id == *room_id {
                    continue;
                }
                if ow.allow.contains(&Permission::ViewChannel) {
                    sequence.push(VisibilityPermission {
                        id: ow.id,
                        ty: ow.ty,
                        allowed: true,
                    });
                }
            }
            // 4. role deny
            for ow in &ow_set {
                if ow.ty != PermissionOverwriteType::Role || ow.id == *room_id {
                    continue;
                }
                if ow.deny.contains(&Permission::ViewChannel) {
                    sequence.push(VisibilityPermission {
                        id: ow.id,
                        ty: ow.ty,
                        allowed: false,
                    });
                }
            }
            // 5. user allow
            for ow in &ow_set {
                if ow.ty != PermissionOverwriteType::User {
                    continue;
                }
                if ow.allow.contains(&Permission::ViewChannel) {
                    sequence.push(VisibilityPermission {
                        id: ow.id,
                        ty: ow.ty,
                        allowed: true,
                    });
                }
            }
            // 6. user deny
            for ow in &ow_set {
                if ow.ty != PermissionOverwriteType::User {
                    continue;
                }
                if ow.deny.contains(&Permission::ViewChannel) {
                    sequence.push(VisibilityPermission {
                        id: ow.id,
                        ty: ow.ty,
                        allowed: false,
                    });
                }
            }
        }

        // prune: only the last overwrite for each (id, ty) matters across the whole inheritance chain
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for ow in sequence.into_iter().rev() {
            if seen.insert((ow.id, ow.ty)) {
                result.push(ow);
            }
        }
        result.reverse();

        Self { overwrites: result }
    }

    /// check if this member can view a channel with this set of overwrites. has_base is if the member can view all channels by default.
    pub fn visible_to(&self, member: &RoomMember, has_base: bool) -> bool {
        let mut has_view = has_base;

        for ow in &self.overwrites {
            match ow.ty {
                PermissionOverwriteType::Role => {
                    if member.roles.contains(&ow.id.into()) || ow.id == *member.room_id {
                        has_view = ow.allowed;
                    }
                }
                PermissionOverwriteType::User => {
                    if ow.id == *member.user_id {
                        has_view = ow.allowed;
                    }
                }
            }
        }

        has_view
    }
}
