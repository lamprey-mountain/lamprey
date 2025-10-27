use async_trait::async_trait;
use common::v1::types::{
    webhook::{Webhook, WebhookCreate, WebhookUpdate},
    ChannelId, RoomId, UserId, WebhookId,
};
use uuid::Uuid;

use crate::{
    data::DataWebhook,
    error::{Error, Result},
};

use super::Postgres;

#[async_trait]
impl DataWebhook for Postgres {
    async fn webhook_create(
        &self,
        channel_id: ChannelId,
        creator_id: UserId,
        create: WebhookCreate,
    ) -> Result<Webhook> {
        let mut tx = self.pool.begin().await?;

        let webhook_id = WebhookId::new();
        let version_id = webhook_id.into_inner();

        sqlx::query!(
            r#"
            INSERT INTO usr (id, version_id, parent_id, name, description, avatar, can_fork, system, registered_at)
            VALUES ($1, $2, $3, $4, $5, $6, false, false, now())
            "#,
            *webhook_id,
            version_id,
            *creator_id,
            create.name,
            Option::<String>::None,
            create.avatar.map(|i| *i)
        )
        .execute(&mut *tx)
        .await?;

        let token: String = Uuid::new_v4().to_string();

        sqlx::query!(
            r#"
            INSERT INTO webhook (id, token, channel_id, creator_id)
            VALUES ($1, $2, $3, $4)
            "#,
            *webhook_id,
            token,
            *channel_id,
            *creator_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.webhook_get(webhook_id).await
    }

    async fn webhook_get(&self, webhook_id: WebhookId) -> Result<Webhook> {
        let row = sqlx::query!(
            r#"
            SELECT w.id, c.room_id, w.channel_id, u.name, u.avatar, w.token
            FROM webhook w
            JOIN usr u ON w.id = u.id
            JOIN channel c ON w.channel_id = c.id
            WHERE w.id = $1
            "#,
            *webhook_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Webhook {
            id: row.id.into(),
            room_id: row.room_id.map(Into::into),
            channel_id: row.channel_id.into(),
            name: row.name,
            avatar: row.avatar.map(Into::into),
            token: Some(row.token),
        })
    }

    async fn webhook_get_with_token(&self, webhook_id: WebhookId, token: &str) -> Result<Webhook> {
        let row = sqlx::query!(
            r#"
            SELECT w.id, c.room_id, w.channel_id, u.name, u.avatar
            FROM webhook w
            JOIN usr u ON w.id = u.id
            JOIN channel c ON w.channel_id = c.id
            WHERE w.id = $1 AND w.token = $2
            "#,
            *webhook_id,
            token
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Webhook {
            id: row.id.into(),
            room_id: row.room_id.map(Into::into),
            channel_id: row.channel_id.into(),
            name: row.name,
            avatar: row.avatar.map(Into::into),
            token: None,
        })
    }

    async fn webhook_list_channel(&self, channel_id: ChannelId) -> Result<Vec<Webhook>> {
        let rows = sqlx::query!(
            r#"
            SELECT w.id, c.room_id, w.channel_id, u.name, u.avatar
            FROM webhook w
            JOIN usr u ON w.id = u.id
            JOIN channel c ON w.channel_id = c.id
            WHERE w.channel_id = $1
            "#,
            *channel_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Webhook {
                id: row.id.into(),
                room_id: row.room_id.map(Into::into),
                channel_id: row.channel_id.into(),
                name: row.name,
                avatar: row.avatar.map(Into::into),
                token: None,
            })
            .collect())
    }

    async fn webhook_list_room(&self, room_id: RoomId) -> Result<Vec<Webhook>> {
        let rows = sqlx::query!(
            r#"
            SELECT w.id, c.room_id, w.channel_id, u.name, u.avatar
            FROM webhook w
            JOIN usr u ON w.id = u.id
            JOIN channel c ON w.channel_id = c.id
            WHERE c.room_id = $1
            "#,
            *room_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Webhook {
                id: row.id.into(),
                room_id: row.room_id.map(Into::into),
                channel_id: row.channel_id.into(),
                name: row.name,
                avatar: row.avatar.map(Into::into),
                token: None,
            })
            .collect())
    }

    async fn webhook_update(&self, webhook_id: WebhookId, patch: WebhookUpdate) -> Result<Webhook> {
        let mut tx = self.pool.begin().await?;

        if let Some(name) = patch.name {
            sqlx::query!("UPDATE usr SET name = $1 WHERE id = $2", name, *webhook_id)
                .execute(&mut *tx)
                .await?;
        }
        if let Some(avatar) = patch.avatar {
            sqlx::query!(
                "UPDATE usr SET avatar = $1 WHERE id = $2",
                avatar.map(|i| *i),
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
        }
        if let Some(channel_id) = patch.channel_id {
            sqlx::query!(
                "UPDATE webhook SET channel_id = $1 WHERE id = $2",
                *channel_id,
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
        }
        if patch.rotate_token {
            let token: String = Uuid::new_v4().to_string();
            sqlx::query!(
                "UPDATE webhook SET token = $1 WHERE id = $2",
                token,
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.webhook_get(webhook_id).await
    }

    async fn webhook_update_with_token(
        &self,
        webhook_id: WebhookId,
        token: &str,
        patch: WebhookUpdate,
    ) -> Result<Webhook> {
        let mut tx = self.pool.begin().await?;

        let res = sqlx::query!(
            "SELECT id FROM webhook WHERE id = $1 AND token = $2",
            *webhook_id,
            token
        )
        .fetch_optional(&mut *tx)
        .await?;

        if res.is_none() {
            return Err(Error::NotFound);
        }

        if let Some(name) = patch.name {
            sqlx::query!("UPDATE usr SET name = $1 WHERE id = $2", name, *webhook_id)
                .execute(&mut *tx)
                .await?;
        }
        if let Some(avatar) = patch.avatar {
            sqlx::query!(
                "UPDATE usr SET avatar = $1 WHERE id = $2",
                avatar.map(|i| *i),
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
        }
        if let Some(channel_id) = patch.channel_id {
            sqlx::query!(
                "UPDATE webhook SET channel_id = $1 WHERE id = $2",
                *channel_id,
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
        }
        if patch.rotate_token {
            let new_token: String = Uuid::new_v4().to_string();
            sqlx::query!(
                "UPDATE webhook SET token = $1 WHERE id = $2",
                new_token,
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.webhook_get(webhook_id).await
    }

    async fn webhook_delete(&self, webhook_id: WebhookId) -> Result<()> {
        sqlx::query!("DELETE FROM usr WHERE id = $1", *webhook_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn webhook_delete_with_token(&self, webhook_id: WebhookId, token: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let res = sqlx::query!(
            "SELECT id FROM webhook WHERE id = $1 AND token = $2",
            *webhook_id,
            token
        )
        .fetch_optional(&mut *tx)
        .await?;
        if res.is_none() {
            return Err(Error::NotFound);
        }
        sqlx::query!("DELETE FROM usr WHERE id = $1", *webhook_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
