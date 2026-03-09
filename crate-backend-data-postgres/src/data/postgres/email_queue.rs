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
        &self,
        to: String,
        from: String,
        subject: String,
        plain_text_body: String,
        html_body: Option<String>,
    ) -> Result<Uuid> {
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
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    async fn email_queue_claim(&self) -> Result<Option<DbEmailQueue>> {
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
        .fetch_optional(&self.pool)
        .await?;
        Ok(email_item)
    }

    async fn email_queue_finish(&self, id: Uuid) -> Result<()> {
        query!(
            r#"
            UPDATE email_queue
            SET status = 'sent', finished_at = NOW()
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn email_queue_fail(&self, error_message: String, id: Uuid) -> Result<()> {
        query!(
            r#"
            UPDATE email_queue
            SET status = 'failed', retries = retries + 1, last_attempt_at = NOW(), error_message = $1
            WHERE id = $2
            "#,
            error_message,
            id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
