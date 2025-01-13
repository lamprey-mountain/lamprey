use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire, PgPool};
use tracing::info;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    Identifier, Media, MediaId, MediaLink, MediaLinkType, Message, MessageCreate, MessageId, MessageType, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse, Permission, Role, RoleCreate, RoleId, Room, RoomCreate, RoomId, RoomMemberPut, RoomPatch, RoomVerId, Thread, ThreadCreate, ThreadId, UserId
};

use crate::data::{
    DataMedia, DataMessage, DataPermission, DataRole, DataRoleMember, DataRoom, DataRoomMember, DataThread, DataUnread
};

use super::Postgres;

#[async_trait]
impl DataRole for Postgres {
    async fn role_create(&self, create: RoleCreate) -> Result<Role> {
        let mut conn = self.pool.acquire().await?;
        let role_id = Uuid::now_v7();
        let role = query_as!(Role, r#"
            INSERT INTO role (id, room_id, name, description, permissions, is_mentionable, is_self_applicable, is_default)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, room_id, name, description, permissions as "permissions: _", is_mentionable, is_self_applicable, is_default
        "#, role_id, create.room_id.into_inner(), create.name, create.description, create.permissions as _, create.is_mentionable, create.is_self_applicable, create.is_default)
    	    .fetch_one(&mut *conn)
        	.await?;
        info!("inserted role");
        Ok(role)
    }

    async fn role_list(&self,room_id: RoomId, paginate: PaginationQuery<RoleId>) -> Result<PaginationResponse<Role>> {
        todo!()
    }

    async fn role_delete(&self,room_id: RoomId, role_id: RoleId) -> Result<()> {
        todo!()
    }

    async fn role_select(&self,room_id: RoomId, role_id: RoleId) -> Result<Role> {
        todo!()
    }

    async fn role_update(&self,room_id: RoomId, role_id: RoleId, patch: crate::types::RolePatch) -> Result<Role> {
        todo!()
    }
}
