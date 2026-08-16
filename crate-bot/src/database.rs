use anyhow::Result;
use common::v1::types::UserId;
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, FromRow)]
pub struct ReminderRow {
    pub id: i64,
    pub text: String,
    pub scheduled_at: String,
}

#[derive(Clone, Debug)]
pub struct BotDatabase {
    pool: SqlitePool,
}

impl BotDatabase {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn add_reminder(
        &self,
        user_id: UserId,
        text: &str,
        scheduled_at: &str,
    ) -> Result<()> {
        let user_id_str = user_id.to_string();
        sqlx::query!(
            r#"
            INSERT INTO reminders (user_id, text, scheduled_at)
            VALUES (?, ?, ?)
            "#,
            user_id_str,
            text,
            scheduled_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_reminder(&self, id: i64) -> Result<()> {
        sqlx::query!("DELETE FROM reminders WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn remove_all_reminders(&self, user_id: UserId) -> Result<()> {
        let user_id_str = user_id.to_string();
        sqlx::query!("DELETE FROM reminders WHERE user_id = ?", user_id_str)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_reminders(&self, user_id: UserId) -> Result<Vec<ReminderRow>> {
        let user_id_str = user_id.to_string();
        let reminders = sqlx::query_as!(
            ReminderRow,
            r#"
            SELECT
              id,
              text,
              scheduled_at
            FROM reminders
            WHERE user_id = ?
            ORDER BY scheduled_at ASC
            "#,
            user_id_str
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(reminders)
    }
}
