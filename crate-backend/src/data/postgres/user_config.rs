use async_trait::async_trait;
use common::v1::types::{
    user_config::{UserConfigGlobal, UserConfigRoom, UserConfigThread, UserConfigUser},
    RoomId, ThreadId,
};
use sqlx::{query, query_scalar};

use crate::error::Result;
use crate::types::UserId;

use crate::data::DataUserConfig;

use super::Postgres;

#[async_trait]
impl DataUserConfig for Postgres {
    async fn user_config_set(&self, user_id: UserId, config: &UserConfigGlobal) -> Result<()> {
        query!(
            "update usr set config = $2 where id = $1",
            *user_id,
            serde_json::to_value(config)?,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn user_config_get(&self, user_id: UserId) -> Result<UserConfigGlobal> {
        let conf = query_scalar!("select config from usr where id = $1", *user_id)
            .fetch_one(&self.pool)
            .await?;
        let conf = conf
            .map(serde_json::from_value)
            .and_then(|v| v.ok())
            .unwrap_or_default();
        Ok(conf)
    }

    async fn user_config_room_set(
        &self,
        user_id: UserId,
        room_id: RoomId,
        config: &UserConfigRoom,
    ) -> Result<()> {
        query!(
            "
            INSERT INTO user_config_room (user_id, room_id, config)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, room_id) DO UPDATE SET config = $3
            ",
            *user_id,
            *room_id,
            serde_json::to_value(config)?,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn user_config_room_get(
        &self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<UserConfigRoom> {
        let conf = query_scalar!(
            "SELECT config FROM user_config_room WHERE user_id = $1 AND room_id = $2",
            *user_id,
            *room_id
        )
        .fetch_optional(&self.pool)
        .await?;
        let conf = conf
            .map(serde_json::from_value)
            .and_then(|v| v.ok())
            .unwrap_or_default();
        Ok(conf)
    }

    async fn user_config_thread_set(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        config: &UserConfigThread,
    ) -> Result<()> {
        query!(
            "
            INSERT INTO user_config_thread (user_id, thread_id, config)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, thread_id) DO UPDATE SET config = $3
            ",
            *user_id,
            *thread_id,
            serde_json::to_value(config)?,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn user_config_thread_get(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
    ) -> Result<UserConfigThread> {
        let conf = query_scalar!(
            "SELECT config FROM user_config_thread WHERE user_id = $1 AND thread_id = $2",
            *user_id,
            *thread_id
        )
        .fetch_optional(&self.pool)
        .await?;
        let conf = conf
            .map(serde_json::from_value)
            .and_then(|v| v.ok())
            .unwrap_or_default();
        Ok(conf)
    }

    async fn user_config_user_set(
        &self,
        user_id: UserId,
        other_id: UserId,
        config: &UserConfigUser,
    ) -> Result<()> {
        query!(
            "
            INSERT INTO user_config_user (user_id, other_id, config)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, other_id) DO UPDATE SET config = $3
            ",
            *user_id,
            *other_id,
            serde_json::to_value(config)?,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn user_config_user_get(
        &self,
        user_id: UserId,
        other_id: UserId,
    ) -> Result<UserConfigUser> {
        let conf = query_scalar!(
            "SELECT config FROM user_config_user WHERE user_id = $1 AND other_id = $2",
            *user_id,
            *other_id
        )
        .fetch_optional(&self.pool)
        .await?;
        let conf = conf
            .map(serde_json::from_value)
            .and_then(|v| v.ok())
            .unwrap_or_default();
        Ok(conf)
    }
}
