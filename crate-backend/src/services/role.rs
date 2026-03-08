use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use common::v1::types::audit_logs::AuditLogEntryType;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::sync::MessageSync;
use common::v1::types::util::Changes;
use common::v1::types::{
    PaginationQuery, PaginationResponse, Permission, Role, RoleCreate, RoleId, RolePatch,
    RoleReorder, RoomId, RoomMember, UserId,
};
use moka::future::Cache;
use validator::Validate;

use crate::error::{Error, Result};
use crate::routes::util::Auth;
use crate::types::DbRoleCreate;
use crate::ServerStateInner;

pub struct ServiceRoles {
    state: Arc<ServerStateInner>,
    idempotency_keys: Cache<String, Role>,
}

impl ServiceRoles {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            idempotency_keys: Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    pub async fn create(
        &self,
        room_id: RoomId,
        auth: &Auth,
        json: RoleCreate,
        nonce: Option<String>,
    ) -> Result<Role> {
        if let Some(n) = &nonce {
            self.idempotency_keys
                .try_get_with(
                    n.clone(),
                    self.create_inner(room_id, auth, json, nonce.clone()),
                )
                .await
                .map_err(|err| err.fake_clone())
        } else {
            self.create_inner(room_id, auth, json, nonce).await
        }
    }

    async fn create_inner(
        &self,
        room_id: RoomId,
        auth: &Auth,
        json: RoleCreate,
        nonce: Option<String>,
    ) -> Result<Role> {
        json.validate()?;
        let data = self.state.data();
        let srv = self.state.services();

        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;

        if room.security.require_sudo {
            auth.ensure_sudo()?;
        }
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }

        let allow_set: HashSet<_> = json.allow.iter().collect();
        let deny_set: HashSet<_> = json.deny.iter().collect();

        if !allow_set.is_disjoint(&deny_set) {
            return Err(ApiError::from_code(ErrorCode::PermissionConflict).into());
        }

        let perms = srv.perms.for_room(auth.user.id, room_id).await?;
        perms.ensure(Permission::RoleManage)?;

        for p in &json.allow {
            perms.ensure(*p)?;
        }

        let room = srv.rooms.get(room_id, None).await?;
        let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
        if rank == 0 && room.owner_id != Some(auth.user.id) {
            return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
        }

        let role = data
            .role_create(
                DbRoleCreate {
                    id: RoleId::new(),
                    room_id,
                    name: json.name,
                    description: json.description,
                    allow: json.allow,
                    deny: json.deny,
                    is_self_applicable: json.is_self_applicable,
                    is_mentionable: json.is_mentionable,
                    hoist: json.hoist,
                    sticky: json.sticky,
                },
                1,
            )
            .await?;

        data.room_template_mark_dirty(room_id).await?;

        let changes = Changes::new()
            .add("name", &role.name)
            .add("description", &role.description)
            .add("allow", &role.allow)
            .add("deny", &role.deny)
            .add("is_self_applicable", &role.is_self_applicable)
            .add("is_mentionable", &role.is_mentionable)
            .add("hoist", &role.hoist)
            .add("sticky", &role.sticky)
            .build();

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::RoleCreate { changes })
            .await?;

        let msg = MessageSync::RoleCreate { role: role.clone() };
        self.state
            .broadcast_room_with_nonce(room_id, auth.user.id, nonce.as_deref(), msg)
            .await?;

        srv.perms.invalidate_user_ranks(room_id);

        Ok(role)
    }

    pub async fn get(&self, room_id: RoomId, role_id: RoleId) -> Result<Role> {
        let roles = self.state.data().role_get_many(room_id, &[role_id]).await?;
        roles
            .into_iter()
            .next()
            .ok_or(Error::ApiError(ApiError::from_code(ErrorCode::UnknownRole)))
    }

    pub async fn update(
        &self,
        room_id: RoomId,
        role_id: RoleId,
        auth: &Auth,
        json: RolePatch,
    ) -> Result<Role> {
        let data = self.state.data();
        let srv = self.state.services();

        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;

        if room.security.require_sudo {
            auth.ensure_sudo()?;
        }
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }

        let perms = srv.perms.for_room(auth.user.id, room_id).await?;
        perms.ensure(Permission::RoleManage)?;

        let role_before = data.role_get_many(room_id, &[role_id]).await?;
        let role_before = role_before
            .into_iter()
            .next()
            .ok_or(Error::ApiError(ApiError::from_code(ErrorCode::UnknownRole)))?;
        data.role_update(room_id, role_id, json).await?;
        data.room_template_mark_dirty(room_id).await?;
        let role = data.role_get_many(room_id, &[role_id]).await?;
        let role = role
            .into_iter()
            .next()
            .ok_or(Error::ApiError(ApiError::from_code(ErrorCode::UnknownRole)))?;

        let changes = Changes::new()
            .change("name", &role_before.name, &role.name)
            .change("description", &role_before.description, &role.description)
            .change("allow", &role_before.allow, &role.allow)
            .change("deny", &role_before.deny, &role.deny)
            .change(
                "is_self_applicable",
                &role_before.is_self_applicable,
                &role.is_self_applicable,
            )
            .change(
                "is_mentionable",
                &role_before.is_mentionable,
                &role.is_mentionable,
            )
            .change("hoist", &role_before.hoist, &role.hoist)
            .change("sticky", &role_before.sticky, &role.sticky)
            .build();

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::RoleUpdate { changes })
            .await?;

        let msg = MessageSync::RoleUpdate { role: role.clone() };
        self.state
            .broadcast_room(room_id, auth.user.id, msg)
            .await?;

        srv.perms.invalidate_user_ranks(room_id);

        Ok(role)
    }

    pub async fn delete(&self, room_id: RoomId, role_id: RoleId, auth: &Auth) -> Result<()> {
        let data = self.state.data();
        let srv = self.state.services();

        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;

        if room.security.require_sudo {
            auth.ensure_sudo()?;
        }
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }

        let perms = srv.perms.for_room(auth.user.id, room_id).await?;
        perms.ensure(Permission::RoleManage)?;

        let roles = data.role_get_many(room_id, &[role_id]).await?;
        let role = roles
            .into_iter()
            .next()
            .ok_or(Error::ApiError(ApiError::from_code(ErrorCode::UnknownRole)))?;

        data.role_delete(room_id, role_id).await?;
        data.room_template_mark_dirty(room_id).await?;

        let changes = Changes::new()
            .remove("name", &role.name)
            .remove("description", &role.description)
            .remove("allow", &role.allow)
            .remove("deny", &role.deny)
            .build();

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::RoleDelete { role_id, changes })
            .await?;

        let msg = MessageSync::RoleDelete { room_id, role_id };
        self.state
            .broadcast_room(room_id, auth.user.id, msg)
            .await?;

        srv.perms.invalidate_user_ranks(room_id);

        Ok(())
    }

    pub async fn reorder(&self, room_id: RoomId, auth: &Auth, reorder: RoleReorder) -> Result<()> {
        let data = self.state.data();
        let srv = self.state.services();

        let perms = srv.perms.for_room(auth.user.id, room_id).await?;
        perms.ensure(Permission::RoleManage)?;

        data.role_reorder(room_id, reorder).await?;
        data.room_template_mark_dirty(room_id).await?;
        srv.perms.invalidate_user_ranks(room_id);

        Ok(())
    }

    pub async fn member_update(
        &self,
        room_id: RoomId,
        role_id: RoleId,
        apply_user_ids: &[UserId],
        remove_user_ids: &[UserId],
        auth: &Auth,
    ) -> Result<()> {
        let data = self.state.data();
        let srv = self.state.services();

        let perms = srv.perms.for_room(auth.user.id, room_id).await?;
        perms.ensure(Permission::RoleManage)?;

        data.role_member_bulk_edit(room_id, role_id, apply_user_ids, remove_user_ids)
            .await?;
        srv.perms.invalidate_user_ranks(room_id);

        Ok(())
    }

    pub async fn list(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<RoleId>,
    ) -> Result<PaginationResponse<Role>> {
        self.state.data().role_list(room_id, pagination).await
    }

    pub async fn member_list(
        &self,
        _room_id: RoomId,
        role_id: RoleId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomMember>> {
        self.state
            .data()
            .role_member_list(role_id, pagination)
            .await
    }
}
