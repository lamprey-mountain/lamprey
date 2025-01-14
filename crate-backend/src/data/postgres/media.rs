use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire, PgPool};
use tracing::info;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    Identifier, Media, MediaId, MediaLink, MediaLinkType, Message, MessageCreate, MessageId,
    MessageType, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse,
    Permission, Role, RoleCreate, RoleId, Room, RoomCreate, RoomId, RoomMemberPut, RoomPatch,
    RoomVerId, Thread, ThreadCreate, ThreadId, UserId,
};

use crate::data::{
    DataMedia, DataMessage, DataPermission, DataRole, DataRoleMember, DataRoom, DataRoomMember,
    DataThread, DataUnread,
};

use super::{Pagination, Postgres};

#[async_trait]
impl DataMedia for Postgres {
    async fn media_insert(&self, user_id: UserId, media: Media) -> Result<Media> {
        let mut conn = self.pool.acquire().await?;
        let media = query_as!(
            Media,
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
            media.size,
            media.mime,
            media.height,
            media.width,
            media.duration,
        )
        .fetch_one(&mut *conn)
        .await?;
        info!("inserted media");
        Ok(media)
    }

    async fn media_select(&self, media_id: MediaId) -> Result<Media> {
        let mut conn = self.pool.acquire().await?;
        let media = query_as!(
            Media,
            "
    	    SELECT id, url, source_url, thumbnail_url, filename, alt, size, mime, height, width, duration
    	    FROM media
    	    WHERE id = $1
        ",
            media_id.into_inner(),
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(media)
    }

    async fn media_link_insert(
        &self,
        media_id: MediaId,
        target_id: Uuid,
        link_type: MediaLinkType,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let links = query_as!(
            MediaLink,
            r#"
    	    INSERT INTO media_link (media_id, target_id, link_type)
    	    VALUES ($1, $2, $3)
    	    RETURNING media_id, target_id, link_type as "link_type: _"
        "#,
            media_id.into_inner(),
            target_id,
            link_type as _
        )
        .fetch_all(&mut *conn)
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
        query!(
            "DELETE FROM media_link WHERE target_id = $1",
            target_id
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }
}
