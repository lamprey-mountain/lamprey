use std::collections::HashMap;

use async_trait::async_trait;
use common::v1::types::{
    ChannelId, RoomId,
    preferences::{PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser},
};
use sqlx::{query, query_scalar};

use crate::error::Result;
use crate::types::UserId;

use crate::data::DataPreferences;

use super::Postgres;

#[async_trait]
impl DataPreferences for Postgres {
    async fn preferences_set(&mut self, user_id: UserId, config: &PreferencesGlobal) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "update usr set config = $2 where id = $1",
            *user_id,
            serde_json::to_value(config)?,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn preferences_get(&mut self, user_id: UserId) -> Result<PreferencesGlobal> {
        let mut conn = self.acquire().await?;
        let conf = query_scalar!("select config from usr where id = $1", *user_id)
            .fetch_one(conn.ext())
            .await?;
        let conf = conf
            .map(serde_json::from_value)
            .and_then(|v| v.ok())
            .unwrap_or_default();
        Ok(conf)
    }

    async fn preferences_room_set(
        &mut self,
        user_id: UserId,
        room_id: RoomId,
        config: &PreferencesRoom,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "
            INSERT INTO preferences_room (user_id, room_id, config)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, room_id) DO UPDATE SET config = $3
            ",
            *user_id,
            *room_id,
            serde_json::to_value(config)?,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn preferences_room_get(
        &mut self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<PreferencesRoom> {
        let mut conn = self.acquire().await?;
        let conf = query_scalar!(
            "SELECT config FROM preferences_room WHERE user_id = $1 AND room_id = $2",
            *user_id,
            *room_id
        )
        .fetch_optional(conn.ext())
        .await?;
        let conf = conf
            .map(serde_json::from_value)
            .and_then(|v| v.ok())
            .unwrap_or_default();
        Ok(conf)
    }

    async fn preferences_room_get_many(
        &mut self,
        user_id: UserId,
        room_ids: &[RoomId],
    ) -> Result<HashMap<RoomId, PreferencesRoom>> {
        let mut conn = self.acquire().await?;
        if room_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let room_ids: Vec<_> = room_ids.iter().map(|id| **id).collect();
        let rows = query!(
            "SELECT room_id, config FROM preferences_room WHERE user_id = $1 AND room_id = ANY($2)",
            *user_id,
            &room_ids[..],
        )
        .fetch_all(conn.ext())
        .await?;

        let mut map = HashMap::with_capacity(room_ids.len());
        for row in rows {
            let room_id: RoomId = row.room_id.into();
            let config: PreferencesRoom = serde_json::from_value(row.config).unwrap_or_default();
            map.insert(room_id, config);
        }

        Ok(map)
    }

    async fn preferences_channel_set(
        &mut self,
        user_id: UserId,
        channel_id: ChannelId,
        config: &PreferencesChannel,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "
            INSERT INTO preferences_channel (user_id, channel_id, config)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, channel_id) DO UPDATE SET config = $3
            ",
            *user_id,
            *channel_id,
            serde_json::to_value(config)?,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn preferences_channel_get(
        &mut self,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<PreferencesChannel> {
        let mut conn = self.acquire().await?;
        let conf = query_scalar!(
            "SELECT config FROM preferences_channel WHERE user_id = $1 AND channel_id = $2",
            *user_id,
            *channel_id,
        )
        .fetch_optional(conn.ext())
        .await?;
        let conf = conf
            .map(serde_json::from_value)
            .and_then(|v| v.ok())
            .unwrap_or_default();
        Ok(conf)
    }

    async fn preferences_channel_get_many(
        &mut self,
        user_id: UserId,
        channel_ids: &[ChannelId],
    ) -> Result<HashMap<ChannelId, PreferencesChannel>> {
        let mut conn = self.acquire().await?;
        if channel_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let channel_ids: Vec<_> = channel_ids.iter().map(|id| **id).collect();
        let rows = query!(
            "SELECT channel_id, config FROM preferences_channel WHERE user_id = $1 AND channel_id = ANY($2)",
            *user_id,
            &channel_ids[..],
        )
        .fetch_all(conn.ext())
        .await?;

        let mut map = HashMap::with_capacity(channel_ids.len());
        for row in rows {
            let channel_id: ChannelId = row.channel_id.into();
            let config: PreferencesChannel = serde_json::from_value(row.config).unwrap_or_default();
            map.insert(channel_id, config);
        }

        Ok(map)
    }

    async fn preferences_user_set(
        &mut self,
        user_id: UserId,
        other_id: UserId,
        config: &PreferencesUser,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "
            INSERT INTO preferences_user (user_id, other_id, config)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, other_id) DO UPDATE SET config = $3
            ",
            *user_id,
            *other_id,
            serde_json::to_value(config)?,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn preferences_user_get(
        &mut self,
        user_id: UserId,
        other_id: UserId,
    ) -> Result<PreferencesUser> {
        let mut conn = self.acquire().await?;
        let conf = query_scalar!(
            "SELECT config FROM preferences_user WHERE user_id = $1 AND other_id = $2",
            *user_id,
            *other_id
        )
        .fetch_optional(conn.ext())
        .await?;
        let conf = conf
            .map(serde_json::from_value)
            .and_then(|v| v.ok())
            .unwrap_or_default();
        Ok(conf)
    }
}
