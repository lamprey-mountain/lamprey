use async_trait::async_trait;
use sqlx::query;

use crate::config::ConfigInternal;
use crate::data::DataConfigInternal;
use crate::error::Result;

use super::Postgres;

#[async_trait]
impl DataConfigInternal for Postgres {
    async fn config_put(&self, config: ConfigInternal) -> Result<()> {
        query!(
            "INSERT INTO config_internal (key, vapid_private_key, vapid_public_key, oidc_jwk_key, admin_token)
             VALUES ('main', $1, $2, $3, $4)
             ON CONFLICT (key) DO UPDATE SET
                vapid_private_key = EXCLUDED.vapid_private_key,
                vapid_public_key = EXCLUDED.vapid_public_key,
                oidc_jwk_key = EXCLUDED.oidc_jwk_key,
                admin_token = EXCLUDED.admin_token",
            config.vapid_private_key,
            config.vapid_public_key,
            config.oidc_jwk_key,
            config.admin_token
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn config_get(&self) -> Result<Option<ConfigInternal>> {
        let row = query!(
            "SELECT vapid_private_key, vapid_public_key, oidc_jwk_key, admin_token
             FROM config_internal WHERE key = 'main'"
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(ConfigInternal {
                vapid_private_key: row.vapid_private_key,
                vapid_public_key: row.vapid_public_key,
                oidc_jwk_key: row.oidc_jwk_key,
                admin_token: row.admin_token,
            }))
        } else {
            Ok(None)
        }
    }
}
