use async_trait::async_trait;
use sqlx::{query_as, Acquire};
use tracing::info;

use crate::error::Result;
use crate::types::{RoleId, UserId};

use crate::data::DataRoleMember;

use super::Postgres;

#[async_trait]
impl DataRoleMember for Postgres {
    async fn role_member_put(&self, user_id: UserId, role_id: RoleId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query_as!(
            Role,
            r#"
            INSERT INTO role_member (user_id, role_id)
    		VALUES ($1, $2)
        "#,
            user_id.into_inner(),
            role_id.into_inner()
        )
        .execute(&mut *conn)
        .await?;
        info!("inserted role member");
        Ok(())
    }

    async fn role_member_delete(&self, user_id: UserId, role_id: RoleId) -> Result<()> {
        todo!()
    }
}
