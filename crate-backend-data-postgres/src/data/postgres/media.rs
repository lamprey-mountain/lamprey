use async_trait::async_trait;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::{Media as MediaV1, MediaTrack as MediaTrackV1};
use common::v2::types::media::{Media as MediaV2, MediaPatch as MediaPatchV2, MediaStatus};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as};
use time::PrimitiveDateTime;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{MediaId, MediaLink, MediaLinkType, UserId};

use crate::data::DataMedia;

use super::Postgres;

#[derive(Debug, Deserialize)]
pub struct DbMedia {
    pub user_id: Uuid,
    pub data: serde_json::Value,
    pub deleted_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct DbMediaWithId {
    pub id: Uuid,
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
    async fn media_insert(&self, media: MediaV2) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let media_id = media.id;
        let user_id = media.user_id.expect("server always has user id");
        let data =
            serde_json::to_value(&DbMediaData::V2(media)).expect("failed to serialize media");
        query!(
            r#"
            INSERT INTO media (id, user_id, data)
            VALUES ($1, $2, $3)
        "#,
            *media_id,
            *user_id,
            data,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        info!("inserted media v2");
        Ok(())
    }

    async fn media_select(&self, media_id: MediaId) -> Result<MediaV2> {
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
        let mut parsed: MediaV2 = serde_json::from_value::<DbMediaData>(media.data)
            .unwrap()
            .into();
        parsed.deleted_at = media.deleted_at.map(Into::into);

        let links = query_as!(
            MediaLink,
            r#"
            SELECT media_id, target_id, link_type as "link_type: _"
            FROM media_link
            WHERE media_id = $1 AND deleted_at IS NULL
        "#,
            *media_id,
        )
        .fetch_all(&self.pool)
        .await?;

        parsed.links = links
            .into_iter()
            .filter_map(|link| {
                use crate::types::MediaLinkType as DbMediaLinkType;
                use common::v2::types::media::MediaLinkType as MediaLinkTypeV2;

                match link.link_type {
                    DbMediaLinkType::Message => match parsed.channel_id {
                        Some(channel_id) => Some(MediaLinkTypeV2::Message {
                            message_id: link.target_id.into(),
                            channel_id,
                        }),
                        // FIXME: populate channel_id for old media
                        None => None,
                    },
                    DbMediaLinkType::MessageVersion => None,
                    DbMediaLinkType::UserAvatar => Some(MediaLinkTypeV2::UserAvatar {
                        user_id: link.target_id.into(),
                    }),
                    DbMediaLinkType::UserBanner => Some(MediaLinkTypeV2::UserBanner {
                        user_id: link.target_id.into(),
                    }),
                    DbMediaLinkType::ChannelIcon => Some(MediaLinkTypeV2::ChannelIcon {
                        channel_id: link.target_id.into(),
                    }),
                    DbMediaLinkType::RoomIcon => Some(MediaLinkTypeV2::RoomIcon {
                        room_id: link.target_id.into(),
                    }),
                    DbMediaLinkType::RoomBanner => Some(MediaLinkTypeV2::RoomBanner {
                        room_id: link.target_id.into(),
                    }),
                    DbMediaLinkType::Embed => Some(MediaLinkTypeV2::Embed {
                        id: link.target_id.into(),
                    }),
                    DbMediaLinkType::CustomEmoji => Some(MediaLinkTypeV2::CustomEmoji {
                        room_id: link.target_id.into(),
                    }),
                }
            })
            .collect();

        if !parsed.links.is_empty() {
            parsed.status = MediaStatus::Consumed;
        }

        Ok(parsed)
    }

    async fn media_update(&self, media_id: MediaId, patch: MediaPatchV2) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let mut media = query_as!(
            DbMedia,
            r#"
            SELECT user_id, deleted_at, data
            FROM media
            WHERE id = $1
            FOR UPDATE
        "#,
            *media_id,
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
        let mut media_data: MediaV2 = media_data.into();

        if let Some(alt) = patch.alt {
            media_data.alt = alt;
        }
        if let Some(filename) = patch.filename {
            media_data.filename = filename;
        }

        if let Some(strip_exif) = patch.strip_exif {
            // Check if media has links (is consumed)
            let links = query_as!(
                MediaLink,
                r#"
                SELECT media_id, target_id, link_type as "link_type: _"
                FROM media_link
                WHERE media_id = $1 AND deleted_at IS NULL
            "#,
                *media_id,
            )
            .fetch_all(&mut *tx)
            .await?;

            if !links.is_empty() {
                return Err(Error::BadStatic(
                    "cannot change strip_exif on consumed media",
                ));
            }

            // Once strip_exif is set to true, it cannot be set to false
            if !strip_exif && media_data.strip_exif {
                return Err(Error::ApiError(ApiError::from_code(
                    ErrorCode::CannotUnsetStripExif,
                )));
            }
            media_data.strip_exif = strip_exif;
        }

        media.data =
            serde_json::to_value(&DbMediaData::V2(media_data)).expect("failed to serialize media");

        query!(
            r#"
            UPDATE media SET
                data = $2
            WHERE id = $1
        "#,
            *media_id,
            media.data,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn media_replace(&self, media: MediaV2) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let media_id = media.id;
        let data =
            serde_json::to_value(&DbMediaData::V2(media)).expect("failed to serialize media");
        query!(
            r#"
            UPDATE media SET
                data = $2
            WHERE id = $1
        "#,
            *media_id,
            data,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        info!("replaced media v2");
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
        let media = self.media_select(media_id).await?;
        if media.status != MediaStatus::Uploaded && media.status != MediaStatus::Consumed {
            return Err(Error::BadStatic("media not uploaded"));
        }

        query!(
            r#"
            INSERT INTO media_link (media_id, target_id, link_type)
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
        "#,
            *media_id,
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
            *media_id,
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
        let row = query!(
            "SELECT id, data FROM media WHERE id = $1 FOR UPDATE",
            media_id.into_inner()
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownMedia,
        )))?; // Ensure media exists

        let media_data: DbMediaData = serde_json::from_value(row.data).expect("invalid data in db");
        let media: MediaV2 = media_data.into();
        if media.status != MediaStatus::Uploaded
            // NOTE: strictly speaking, exclusive links shouldn't happen on consumed media usually, but we allow it for consistency
            && media.status != MediaStatus::Consumed
        {
            return Err(Error::BadStatic("media not uploaded"));
        }

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
            return Err(Error::BadStatic("media already used"));
        }

        query!(
            r#"
            INSERT INTO media_link (media_id, target_id, link_type)
            VALUES ($1, $2, $3)
        "#,
            *media_id,
            target_id,
            link_type as _
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn media_migrate_batch(&self, limit: u32) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let rows = query_as!(
            DbMediaWithId,
            r#"
            select id, user_id, data, deleted_at
            from media
            where (data->>'v' is null or data->>'v' = 'V1') and deleted_at is null
            limit $1
            for update
            "#,
            limit as i64
        )
        .fetch_all(&mut *tx)
        .await?;

        if rows.is_empty() {
            return Ok(0);
        }

        let count = rows.len() as u64;
        for row in rows {
            let media: DbMediaData = match serde_json::from_value(row.data) {
                Ok(media) => media,
                Err(err) => {
                    warn!(media_id = ?row.id, "unreadable data in db {err:?}");
                    continue;
                }
            };
            let media: MediaV2 = media.into();
            let media_id = media.id;
            let data =
                serde_json::to_value(&DbMediaData::V2(media)).expect("failed to serialize media");
            query!("update media set data = $1 where id = $2", data, *media_id)
                .execute(&mut *tx)
                .await?;
            info!("migrate {}", media_id);
        }
        tx.commit().await?;
        Ok(count)
    }
}
