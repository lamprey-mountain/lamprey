use async_trait::async_trait;
use common::v1::types::user_config::UserConfig;
use sqlx::{query, query_scalar};

use crate::error::Result;
use crate::types::UserId;

use crate::data::DataUserConfig;

use super::Postgres;

#[async_trait]
impl DataUserConfig for Postgres {
    async fn user_config_set(&self, user_id: UserId, config: &UserConfig) -> Result<()> {
        query!(
            "update usr set config = $2 where id = $1",
            user_id.into_inner(),
            serde_json::to_value(config)?,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn user_config_get(&self, user_id: UserId) -> Result<UserConfig> {
        let conf = query_scalar!("select config from usr where id = $1", user_id.into_inner(),)
            .fetch_one(&self.pool)
            .await?;
        let conf = conf
            .map(serde_json::from_value)
            .and_then(|v| v.ok())
            .unwrap_or_default();
        Ok(conf)
    }
}
