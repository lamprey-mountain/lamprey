use async_trait::async_trait;
use sqlx::query;

use crate::data::DataConfigInternal;
use crate::error::Result;
use crate::config::ConfigInternal;

use super::Postgres;

#[async_trait]
impl DataConfigInternal for Postgres {
    async fn config_put(&self, config: ConfigInternal) -> Result<()> {
        let value = serde_json::to_value(&config)?;
        query!(
            "INSERT INTO config_internal (key, value) VALUES ('main', $1)
             ON CONFLICT (key) DO UPDATE SET value = $1",
            value
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn config_get(&self) -> Result<Option<ConfigInternal>> {
        let row = query!(
            "SELECT value FROM config_internal WHERE key = 'main'"
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(serde_json::from_value(row.value)?))
        } else {
            Ok(None)
        }
    }
}
