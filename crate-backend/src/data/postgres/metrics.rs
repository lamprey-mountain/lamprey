use async_trait::async_trait;
use sqlx::query_as;

use crate::data::{DataMetrics, InstanceMetrics};
use crate::error::Result;

use super::Postgres;

#[async_trait]
impl DataMetrics for Postgres {
    async fn get_metrics(&self) -> Result<InstanceMetrics> {
        let metrics = query_as!(
            InstanceMetrics,
            r#"
            SELECT
                (SELECT count(*) FROM usr) AS "user_count_total!",
                (SELECT count(*) FROM usr WHERE registered_at IS NULL AND id NOT IN (SELECT id FROM puppet) AND id NOT IN (SELECT id FROM application) AND id NOT IN (SELECT id FROM webhook)) AS "user_count_guest!",
                (SELECT count(*) FROM usr WHERE registered_at IS NOT NULL) AS "user_count_registered!",
                (SELECT count(*) FROM usr WHERE id IN (SELECT id FROM application)) AS "user_count_bot!",
                (SELECT count(*) FROM usr WHERE id IN (SELECT id FROM webhook)) AS "user_count_webhook!",
                (SELECT count(*) FROM usr WHERE id IN (SELECT id FROM puppet)) AS "user_count_puppet!",
                (SELECT count(*) FROM usr WHERE id IN (SELECT id FROM puppet) AND parent_id IN (SELECT id FROM application)) AS "user_count_puppet_bot!",
                (SELECT count(*) FROM room) AS "room_count_total!",
                (SELECT count(*) FROM room WHERE public = false) AS "room_count_private!",
                (SELECT count(*) FROM room WHERE public = true) AS "room_count_public!",
                (SELECT count(*) FROM channel) AS "channel_count_total!",
                (SELECT count(*) FROM channel WHERE type = 'Text') AS "channel_count_text!",
                (SELECT count(*) FROM channel WHERE type = 'Voice') AS "channel_count_voice!",
                (SELECT count(*) FROM channel WHERE type = 'Broadcast') AS "channel_count_broadcast!",
                (SELECT count(*) FROM channel WHERE type = 'Calendar') AS "channel_count_calendar!",
                (SELECT count(*) FROM channel WHERE type = 'ThreadPublic') AS "channel_count_thread_public!",
                (SELECT count(*) FROM channel WHERE type = 'ThreadPrivate') AS "channel_count_thread_private!",
                (SELECT count(*) FROM channel WHERE type = 'Dm') AS "channel_count_dm!",
                (SELECT count(*) FROM channel WHERE type = 'Gdm') AS "channel_count_gdm!"
            "#
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(metrics)
    }
}
