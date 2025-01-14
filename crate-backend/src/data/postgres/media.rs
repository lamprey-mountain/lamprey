use async_trait::async_trait;
use sqlx::{query, query_as};
use tracing::info;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{Media, MediaId, MediaLink, MediaLinkType, MediaRow, UserId};

use crate::data::DataMedia;

use super::Postgres;

#[async_trait]
impl DataMedia for Postgres {
    async fn media_insert(&self, user_id: UserId, media: Media) -> Result<Media> {
        let mut conn = self.pool.acquire().await?;
        let size: i64 = media.size.try_into().expect("too big!");
        let height: Option<i64> = media.height.map(|i| i.try_into().expect("too big!"));
        let width: Option<i64> = media.width.map(|i| i.try_into().expect("too big!"));
        let duration: Option<i64> = media.duration.map(|i| i.try_into().expect("too big!"));
        let media = query_as!(
            MediaRow,
            "
    	    INSERT INTO media (id, user_id, url, source_url, thumbnail_url, filename, alt, size, mime, height, width, duration)
    	    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
    	    RETURNING id, url, source_url, thumbnail_url, filename, alt, size, mime, height, width, duration
        ",
            media.id.into_inner(),
            user_id.into_inner(),
            media.url,
            media.source_url,
            media.thumbnail_url,
            media.filename,
            media.alt,
            size,
            media.mime,
            height,
            width,
            duration,
        )
        .fetch_one(&mut *conn)
        .await?;
        info!("inserted media");
        Ok(media.into())
    }

    async fn media_select(&self, media_id: MediaId) -> Result<Media> {
        let mut conn = self.pool.acquire().await?;
        let media = query_as!(
            MediaRow,
            "
    	    SELECT id, url, source_url, thumbnail_url, filename, alt, size, mime, height, width, duration
    	    FROM media
    	    WHERE id = $1
        ",
            media_id.into_inner(),
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(media.into())
    }

    async fn media_link_insert(
        &self,
        media_id: MediaId,
        target_id: Uuid,
        link_type: MediaLinkType,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query!(
            r#"
    	    INSERT INTO media_link (media_id, target_id, link_type)
    	    VALUES ($1, $2, $3)
        "#,
            media_id.into_inner(),
            target_id,
            link_type as _
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    async fn media_link_select(&self, media_id: MediaId) -> Result<Vec<MediaLink>> {
        let mut conn = self.pool.acquire().await?;
        let links = query_as!(
            MediaLink,
            r#"
    	    SELECT media_id, target_id, link_type as "link_type: _"
    	    FROM media_link
    	    WHERE media_id = $1
        "#,
            media_id.into_inner(),
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(links)
    }

    async fn media_link_delete(&self, target_id: Uuid, link_type: MediaLinkType) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query!(
            "DELETE FROM media_link WHERE target_id = $1 AND link_type = $2",
            target_id,
            link_type as _
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    async fn media_link_delete_all(&self, target_id: Uuid) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query!("DELETE FROM media_link WHERE target_id = $1", target_id)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}
