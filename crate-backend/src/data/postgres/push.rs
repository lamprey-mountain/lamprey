use async_trait::async_trait;
use sqlx::{query, query_as};
use uuid::Uuid;

use crate::data::DataPush;
use crate::error::Result;
use crate::types::{PushData, SessionId, UserId};
use crate::Error;

use super::Postgres;

// Database representation of PushData with raw UUIDs
#[derive(sqlx::FromRow)]
struct DbPushData {
    session_id: Uuid,
    user_id: Uuid,
    endpoint: String,
    key_p256dh: String,
    key_auth: String,
}

impl From<DbPushData> for PushData {
    fn from(db: DbPushData) -> Self {
        PushData {
            session_id: db.session_id.into(),
            user_id: db.user_id.into(),
            endpoint: db.endpoint,
            key_p256dh: db.key_p256dh,
            key_auth: db.key_auth,
        }
    }
}

impl From<&PushData> for DbPushData {
    fn from(push: &PushData) -> Self {
        DbPushData {
            session_id: (*push.session_id).into(),
            user_id: (*push.user_id).into(),
            endpoint: push.endpoint.clone(),
            key_p256dh: push.key_p256dh.clone(),
            key_auth: push.key_auth.clone(),
        }
    }
}

#[async_trait]
impl DataPush for Postgres {
    async fn push_insert(&self, push: PushData) -> Result<()> {
        let db_push: DbPushData = (&push).into();

        query!(
            r#"
            INSERT INTO push_subscription (session_id, user_id, endpoint, key_p256dh, key_auth)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            db_push.session_id,
            db_push.user_id,
            db_push.endpoint,
            db_push.key_p256dh,
            db_push.key_auth
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn push_get(&self, session_id: SessionId) -> Result<PushData> {
        let row = query_as!(
            DbPushData,
            r#"
            SELECT
                session_id,
                user_id,
                endpoint,
                key_p256dh,
                key_auth
            FROM push_subscription
            WHERE session_id = $1
            "#,
            *session_id,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    async fn push_delete(&self, session_id: SessionId) -> Result<()> {
        query!(
            r#"
            DELETE FROM push_subscription
            WHERE session_id = $1
            "#,
            *session_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn push_list_for_user(&self, user_id: UserId) -> Result<Vec<PushData>> {
        let rows = query_as!(
            DbPushData,
            r#"
            SELECT
                session_id,
                user_id,
                endpoint,
                key_p256dh,
                key_auth
            FROM push_subscription
            WHERE user_id = $1
            "#,
            *user_id,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(PushData::from).collect())
    }

    async fn push_delete_for_user(&self, user_id: UserId) -> Result<()> {
        query!(
            r#"
            DELETE FROM push_subscription
            WHERE user_id = $1
            "#,
            *user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
