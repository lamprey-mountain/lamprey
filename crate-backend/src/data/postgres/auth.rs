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
        can_auth: bool,
    ) -> Result<()> {
        query!(
            "INSERT INTO oauth (provider, user_id, remote_id, can_auth) VALUES ($1, $2, $3, $4)",
            provider,
            user_id.into_inner(),
            remote_id,
            can_auth,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn auth_oauth_get_all(&self, user_id: UserId) -> Result<Vec<String>> {
        let providers = query_scalar!("SELECT provider FROM oauth WHERE user_id = $1", *user_id,)
            .fetch_all(&self.pool)
            .await?;
        Ok(providers)
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

    async fn auth_password_set(&self, user_id: UserId, hash: &[u8], salt: &[u8]) -> Result<()> {
        sqlx::query!(
            "update usr set password_hash = $2, password_salt = $3 where id = $1",
            *user_id,
            hash,
            salt
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn auth_password_get(&self, user_id: UserId) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
        let row = sqlx::query!(
            "select password_hash, password_salt from usr where id = $1",
            *user_id,
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else { return Ok(None) };
        match (row.password_hash, row.password_salt) {
            (Some(hash), Some(salt)) => Ok(Some((hash, salt))),
            _ => Ok(None),
        }
    }

    async fn auth_password_delete(&self, user_id: UserId) -> Result<()> {
        sqlx::query!(
            "update usr set password_hash = null, password_salt = null where id = $1",
            *user_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
