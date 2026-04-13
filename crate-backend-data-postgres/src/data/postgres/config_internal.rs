use async_trait::async_trait;
use lamprey_backend_core::config::ServerKeyInternal;
use sqlx::query;
use tracing::warn;

use crate::data::DataConfigInternal;
use crate::error::Result;
use lamprey_backend_core::config::ConfigInternal;

use super::Postgres;

#[async_trait]
impl DataConfigInternal for Postgres {
    async fn config_put(&self, config: ConfigInternal) -> Result<()> {
        query!(
            "INSERT INTO config_internal (key, vapid_private_key, vapid_public_key, oidc_jwk_key, admin_token, federation_keys)
             VALUES ('main', $1, $2, $3, $4, $5)
             ON CONFLICT (key) DO UPDATE SET
                vapid_private_key = EXCLUDED.vapid_private_key,
                vapid_public_key = EXCLUDED.vapid_public_key,
                oidc_jwk_key = EXCLUDED.oidc_jwk_key,
                admin_token = EXCLUDED.admin_token,
                federation_keys = EXCLUDED.federation_keys",
            config.vapid_private_key,
            config.vapid_public_key,
            config.oidc_jwk_key,
            config.admin_token,
            serde_json::to_value(&config.federation_keys)?,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn config_get(&self) -> Result<Option<ConfigInternal>> {
        let row = query!(
            "SELECT vapid_private_key, vapid_public_key, oidc_jwk_key, admin_token, federation_keys
             FROM config_internal WHERE key = 'main'"
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let federation_keys: Vec<ServerKeyInternal> = match row.federation_keys {
                Some(k) => serde_json::from_value(k).unwrap_or_else(|e| {
                    warn!("failed to parse stored federation keys, using empty list: {e}");
                    Vec::new()
                }),
                None => Vec::new(),
            };

            Ok(Some(ConfigInternal {
                vapid_private_key: row.vapid_private_key,
                vapid_public_key: row.vapid_public_key,
                oidc_jwk_key: row.oidc_jwk_key,
                admin_token: row.admin_token,
                federation_keys,
            }))
        } else {
            Ok(None)
        }
    }
}
