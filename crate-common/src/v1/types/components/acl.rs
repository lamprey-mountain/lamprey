#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{
    Permission, RoleId, RoomMember, User, UserId,
    components::{Component, ComponentState, Components},
    error::{ApiError, ApiResult, ErrorCode},
    interactions::InteractionCreate,
};

/// a restriction on who can interact with this component
///
/// *any* of the checks must pass (checks are or'd, not anded). if all of the fields are empty, nobody can interact.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Allow {
    // TODO: deduplicate items in vecs
    /// only these users can interact
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub user_ids: Vec<UserId>,

    /// only these users with these roles can interact
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub role_ids: Vec<RoleId>,

    /// only these users with these permissions can interact
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<Permission>,
}

/// utility to check whether an interaction is allowed
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
