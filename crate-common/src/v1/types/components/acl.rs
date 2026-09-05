use lamprey_macros::record;

use crate::v1::types::{
    Permission, RoleId, RoomMember, User, UserId,
    components::{Component, ComponentState, Components},
    error::{ApiError, ApiResult, ErrorCode},
    interactions::InteractionCreate,
};

/// a restriction on who can interact with this component
///
/// *any* of the checks must pass (checks are or'd, not anded). if all of the fields are empty, nobody can interact.
#[record]
#[derive(Default, PartialEq, Eq)]
pub struct Allow {
    // TODO: deduplicate items in vecs (maybe use HashSet?)
    /// only these users can interact
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_ids: Vec<UserId>,

    /// only these users with these roles can interact
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub role_ids: Vec<RoleId>,

    /// only these users with these permissions can interact
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<Permission>,
}

impl Allow {
    /// deny everyone from using this component
    pub fn none() -> Self {
        Self::default()
    }

    /// allow a user to interact with this component
    pub fn user_id(mut self, user_id: UserId) -> Self {
        self.user_ids.push(user_id);
        self
    }

    /// allow users with this role to interact with this component
    pub fn role_id(mut self, role_id: RoleId) -> Self {
        self.role_ids.push(role_id);
        self
    }

    /// allow users with this permission to interact with this component
    pub fn permission(mut self, permission: Permission) -> Self {
        self.permissions.push(permission);
        self
    }
}

impl From<UserId> for Allow {
    fn from(user_id: UserId) -> Self {
        Self {
            user_ids: vec![user_id],
            ..Default::default()
        }
    }
}

impl From<RoleId> for Allow {
    fn from(role_id: RoleId) -> Self {
        Self {
            role_ids: vec![role_id],
            ..Default::default()
        }
    }
}

impl From<Permission> for Allow {
    fn from(permission: Permission) -> Self {
        Self {
            permissions: vec![permission],
            ..Default::default()
        }
    }
}

impl From<Vec<UserId>> for Allow {
    fn from(user_ids: Vec<UserId>) -> Self {
        Self {
            user_ids,
            ..Default::default()
        }
    }
}

impl From<Vec<RoleId>> for Allow {
    fn from(role_ids: Vec<RoleId>) -> Self {
        Self {
            role_ids,
            ..Default::default()
        }
    }
}

impl From<Vec<Permission>> for Allow {
    fn from(permissions: Vec<Permission>) -> Self {
        Self {
            permissions,
            ..Default::default()
        }
    }
}

/// utility to check whether an interaction is allowed
// TODO: merge into Requirements when that gets implemented
#[derive(Debug)]
pub struct AllowCheck<'a> {
    pub interaction_create: &'a InteractionCreate,
    pub room_member: &'a RoomMember,
    pub user: &'a User,
    pub permissions: Vec<Permission>,
}

impl<'a> AllowCheck<'a> {
    /// check whether this interaction can be applied to these components
    pub fn check<C: ComponentState>(&self, components: &Components<C>) -> ApiResult<()> {
        for component in &components.inner {
            self.check_component(component)?;
        }
        Ok(())
    }

    /// check whether this interaction can be applied to this component
    pub fn check_component<C: ComponentState>(&self, component: &Component<C>) -> ApiResult<()> {
        let Some(allow) = &component.allow else {
            return Ok(());
        };

        // check user ids
        if allow.user_ids.contains(&self.user.id) {
            return Ok(());
        }

        // check role ids
        if !allow.role_ids.is_empty() {
            let user_has_role = self
                .room_member
                .roles
                .iter()
                .any(|role_id| allow.role_ids.contains(role_id));
            if user_has_role {
                return Ok(());
            }
        }

        // check permissions
        if !allow.permissions.is_empty() {
            let user_has_permission = allow
                .permissions
                .iter()
                .any(|permission| self.permissions.contains(permission));
            if user_has_permission {
                return Ok(());
            }
        }

        Err(ApiError::with_message(
            ErrorCode::InteractionNotAllowed,
            "user does not have access to interact with this component".to_owned(),
        ))
    }
}
