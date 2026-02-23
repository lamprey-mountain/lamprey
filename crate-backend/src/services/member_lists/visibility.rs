use common::v1::types::{
    Permission, PermissionOverwrite, PermissionOverwriteType, RoomId, RoomMember,
};
use std::collections::HashSet;
use uuid::Uuid;

/// Minimal calculated visibility for member lists
#[derive(Debug, Default, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemberListVisibility {
    /// flat list of minimal permission overwrites in application order
    overwrites: Vec<VisibilityPermission>,
}

/// Minimal permission overwrite for ViewChannel
#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VisibilityPermission {
    /// ID of role or user
    pub id: Uuid,

    /// Whether this is for a user or role
    pub ty: PermissionOverwriteType,

    /// True if ViewChannel is allowed, false if ViewChannel is denied
    pub allowed: bool,
}

impl MemberListVisibility {
    /// Create visibility from permission overwrites
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

    /// Check if a member can see the list
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
