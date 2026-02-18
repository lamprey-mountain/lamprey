use async_trait::async_trait;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::media::MediaWithAdmin;
use common::v1::types::{Media as MediaV1, MediaPatch as MediaPatchV1, MediaTrack as MediaTrackV1};
use common::v2::types::media::Media as MediaV2;
use lamprey_backend_core::Error;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as};
use time::PrimitiveDateTime;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::Result;
use crate::types::{MediaId, MediaLink, MediaLinkType, UserId};

use crate::data::DataMedia;

use super::Postgres;

#[derive(Debug, Deserialize)]
pub struct DbMedia {
    pub user_id: Uuid,
    pub data: serde_json::Value,
    pub deleted_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "v")]
pub enum DbMediaData {
    V1(MediaV1),

    V2(MediaV2),

    #[serde(untagged)]
    Raw(DbMediaRaw),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbMediaRaw {
    id: MediaId,
    user_id: UserId,
    filename: String,
    alt: Option<String>,
    tracks: Vec<MediaTrackV1>,
}

impl From<DbMediaData> for MediaV1 {
    fn from(value: DbMediaData) -> Self {
        match value {
            DbMediaData::V1(media) => media,
            DbMediaData::V2(media) => media.into(),
            DbMediaData::Raw(db_media_raw) => db_media_raw.into(),
        }
    }
}

impl From<DbMediaData> for MediaV2 {
    fn from(value: DbMediaData) -> Self {
        match value {
            DbMediaData::V1(media) => media.into(),
            DbMediaData::V2(media) => media,
            DbMediaData::Raw(db_media_raw) => {
                let v1: MediaV1 = db_media_raw.into();
                v1.into()
            }
        }
    }
}

impl From<DbMediaRaw> for MediaV1 {
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
            .or_else(|| tracks.first())
            .expect("media should always have at least one track")
            .clone();

        MediaV1 {
            id: value.id,
            filename: value.filename,
            alt: value.alt,
            source,
        }
    }
}

#[async_trait]
impl DataMedia for Postgres {
    async fn media_insert(&self, user_id: UserId, media: MediaV1) -> Result<()> {
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

    async fn media_select(&self, media_id: MediaId) -> Result<MediaWithAdmin> {
        let media = query_as!(
            DbMedia,
            r#"
    	    SELECT user_id, deleted_at, data
    	    FROM media
    	    WHERE id = $1
        "#,
            *media_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                Error::ApiError(ApiError::from_code(ErrorCode::UnknownMedia))
            }
            e => Error::Sqlx(e),
        })?;
        let parsed: DbMediaData = serde_json::from_value(media.data).unwrap();
        Ok(MediaWithAdmin {
            inner: parsed.into(),
            user_id: media.user_id.into(),
            deleted_at: media.deleted_at.map(|t| t.into()),
        })
    }

    async fn media_update(&self, media_id: MediaId, patch: MediaPatchV1) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let mut media = query_as!(
            DbMedia,
            r#"
    	    SELECT user_id, deleted_at, data
    	    FROM media
    	    WHERE id = $1
    	    FOR UPDATE
        "#,
            media_id.into_inner(),
        )
        .fetch_one(&mut *tx)
        .await?;

        if media.deleted_at.is_some() {
            warn!("tried to update media, but media is deleted. ignoring update.");
            tx.rollback().await?;
            return Ok(());
        }

        let media_data: DbMediaData =
            serde_json::from_value(media.data).expect("invalid data in db");
        let mut media_data: MediaV1 = media_data.into();
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
        .ok_or(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownMedia,
        )))?; // Ensure media exists

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
