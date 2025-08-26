use common::v1::types::{EmojiId, MediaId};
use sqlx::{query_scalar, Executor, Postgres};

use crate::error::Result;

pub async fn lookup_emoji<'e, E>(exec: E, emoji_id: EmojiId) -> Result<MediaId>
where
    E: Executor<'e, Database = Postgres>,
{
    let media_id: MediaId =
        query_scalar!("SELECT media_id FROM custom_emoji WHERE id = $1", *emoji_id)
            .fetch_one(exec)
            .await?
            .into();
    Ok(media_id)
}
