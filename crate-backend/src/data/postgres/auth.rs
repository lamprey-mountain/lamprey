use async_trait::async_trait;
use sqlx::{query, query_scalar};

use crate::error::Result;
use crate::types::UserId;

use crate::data::DataAuth;

use super::Postgres;

#[async_trait]
impl DataAuth for Postgres {
    async fn auth_oauth_put(
        &self,
        provider: String,
        user_id: UserId,
        remote_id: String,
    ) -> Result<()> {
        query!(
            "INSERT INTO oauth (provider, user_id, remote_id) VALUES ($1, $2, $3)",
            provider,
            user_id.into_inner(),
            remote_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn auth_oauth_get_remote(&self, provider: String, remote_id: String) -> Result<UserId> {
        let remote_id = query_scalar!(
            "SELECT user_id FROM oauth WHERE remote_id = $1 AND provider = $2",
            remote_id,
            provider,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(remote_id.into())
    }

    async fn auth_oauth_delete(&self, provider: String, user_id: UserId) -> Result<()> {
        query!(
            "DELETE FROM oauth WHERE provider = $1 AND user_id = $2",
            provider,
            user_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
