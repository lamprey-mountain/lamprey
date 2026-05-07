use async_trait::async_trait;
use sqlx::{query, query_as};
use uuid::Uuid;

use crate::data::DataEmailQueue;
use crate::error::Result;
use crate::types::DbEmailQueue;

use super::Postgres;

#[async_trait]
impl DataEmailQueue for Postgres {
    async fn email_queue_insert(
        &mut self,
        to: String,
        from: String,
        subject: String,
        plain_text_body: String,
        html_body: Option<String>,
    ) -> Result<Uuid> {
        let mut conn = self.acquire().await?;
        let id = Uuid::new_v4();
        query!(
            r#"
            INSERT INTO email_queue (id, to_addr, from_addr, subject, plain_text_body, html_body)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            id,
            to,
            from,
            subject,
            plain_text_body,
            html_body
        )
        .execute(conn.ext())
        .await?;
        Ok(id)
    }

    async fn email_queue_claim(&mut self) -> Result<Option<DbEmailQueue>> {
        let mut conn = self.acquire().await?;
        let email_item = query_as!(
            DbEmailQueue,
            r#"
            UPDATE email_queue
            SET status = 'claimed', claimed_at = NOW()
            WHERE id = (
                SELECT id
                FROM email_queue
                WHERE status = 'pending' OR (status = 'failed' AND retries < 5 AND last_attempt_at < NOW() - INTERVAL '5 minutes')
                ORDER BY created_at ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING id, to_addr, from_addr, subject, plain_text_body, html_body
            "#
        )
        .fetch_optional(conn.ext())
        .await?;
        Ok(email_item)
    }

    async fn email_queue_finish(&mut self, id: Uuid) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"
            UPDATE email_queue
            SET status = 'sent', finished_at = NOW()
            WHERE id = $1
            "#,
            id
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn email_queue_fail(&mut self, error_message: String, id: Uuid) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"
            UPDATE email_queue
            SET status = 'failed', retries = retries + 1, last_attempt_at = NOW(), error_message = $1
            WHERE id = $2
            "#,
            error_message,
            id,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }
}
