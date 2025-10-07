use async_trait::async_trait;
use common::v1::types::{MediaPatch, MediaTrack};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as};
use tracing::info;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{Media, MediaId, MediaLink, MediaLinkType, UserId};

use crate::data::DataMedia;

use super::Postgres;

#[derive(Debug, Deserialize)]
pub struct DbMedia {
    pub user_id: Uuid,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "v")]
pub enum DbMediaData {
    V1(Media),

    #[serde(untagged)]
    Raw(DbMediaRaw),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbMediaRaw {
    id: MediaId,
    user_id: UserId,
    filename: String,
    alt: Option<String>,
    tracks: Vec<MediaTrack>,
}

impl From<DbMediaData> for Media {
    fn from(value: DbMediaData) -> Self {
        match value {
            DbMediaData::V1(media) => media,
            DbMediaData::Raw(db_media_raw) => db_media_raw.into(),
        }
    }
}

impl From<DbMediaRaw> for Media {
    fn from(value: DbMediaRaw) -> Self {
        let tracks = value.tracks;

        let source = tracks
            .iter()
            .find(|i| {
                matches!(
                    i.source,
                    common::v1::types::TrackSource::Uploaded
                        | common::v1::types::TrackSource::Downloaded { .. }
                )
            })
            .or_else(|| tracks.get(0))
            .expect("media should always have at least one track")
            .clone();

        Media {
            id: value.id,
            filename: value.filename,
            alt: value.alt,
            source,
        }
    }
}

#[async_trait]
impl DataMedia for Postgres {
    async fn media_insert(&self, user_id: UserId, media: Media) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let media_id = media.id;
        let data =
            serde_json::to_value(&DbMediaData::V1(media)).expect("failed to serialize media");
        query!(
            r#"
    	    INSERT INTO media (id, user_id, data)
    	    VALUES ($1, $2, $3)
        "#,
            media_id.into_inner(),
            user_id.into_inner(),
            data,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        info!("inserted media");
        Ok(())
    }

    async fn media_select(&self, media_id: MediaId) -> Result<(Media, UserId)> {
        let media = query_as!(
            DbMedia,
            r#"
    	    SELECT user_id, data
    	    FROM media
    	    WHERE id = $1
        "#,
            media_id.into_inner(),
        )
        .fetch_one(&self.pool)
        .await?;
        let parsed: DbMediaData = serde_json::from_value(media.data).unwrap();
        Ok((parsed.into(), media.user_id.into()))
    }

    async fn media_update(&self, media_id: MediaId, patch: MediaPatch) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let mut media = query_as!(
            DbMedia,
            r#"
    	    SELECT user_id, data
    	    FROM media
    	    WHERE id = $1
    	    FOR UPDATE
        "#,
            media_id.into_inner(),
        )
        .fetch_one(&mut *tx)
        .await?;
        let media_data: DbMediaData =
            serde_json::from_value(media.data).expect("invalid data in db");
        let mut media_data: Media = media_data.into();
        media_data.alt = patch.alt.flatten();
        media.data = serde_json::to_value(media_data).expect("failed to serialize media");

        query!(
            r#"
    	    UPDATE media SET
        	    data = $2
    	    WHERE id = $1
        "#,
            media_id.into_inner(),
            media.data,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn media_delete(&self, media_id: MediaId) -> Result<()> {
        query!(
            "UPDATE media SET deleted_at = now() WHERE id = $1",
            *media_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn media_link_insert(
        &self,
        media_id: MediaId,
        target_id: Uuid,
        link_type: MediaLinkType,
    ) -> Result<()> {
        query!(
            r#"
    	    INSERT INTO media_link (media_id, target_id, link_type)
    	    VALUES ($1, $2, $3)
    	    ON CONFLICT DO NOTHING
        "#,
            media_id.into_inner(),
            target_id,
            link_type as _
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn media_link_select(&self, media_id: MediaId) -> Result<Vec<MediaLink>> {
        let links = query_as!(
            MediaLink,
            r#"
    	    SELECT media_id, target_id, link_type as "link_type: _"
    	    FROM media_link
    	    WHERE media_id = $1
        "#,
            media_id.into_inner(),
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(links)
    }

    async fn media_link_delete(&self, target_id: Uuid, link_type: MediaLinkType) -> Result<()> {
        query!(
            "DELETE FROM media_link WHERE target_id = $1 AND link_type = $2",
            target_id,
            link_type as _
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn media_link_delete_all(&self, target_id: Uuid) -> Result<()> {
        query!("DELETE FROM media_link WHERE target_id = $1", target_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn media_link_create_exclusive(
        &self,
        media_id: MediaId,
        target_id: Uuid,
        link_type: MediaLinkType,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Lock the media row to serialize access to linking this media
        query!(
            "SELECT id FROM media WHERE id = $1 FOR UPDATE",
            media_id.into_inner()
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(crate::error::Error::NotFound)?; // Ensure media exists

        let links = query_as!(
            MediaLink,
            r#"
    	    SELECT media_id, target_id, link_type as "link_type: _"
    	    FROM media_link
    	    WHERE media_id = $1
        "#,
            media_id.into_inner(),
        )
        .fetch_all(&mut *tx)
        .await?;

        if !links.is_empty() {
            return Err(crate::error::Error::BadStatic("media already used"));
        }

        query!(
            r#"
    	    INSERT INTO media_link (media_id, target_id, link_type)
    	    VALUES ($1, $2, $3)
        "#,
            media_id.into_inner(),
            target_id,
            link_type as _
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}
