use async_trait::async_trait;
use serde::Deserialize;
use sqlx::{query, query_as};
use tracing::info;
use types::{MediaSize, MediaTrack, MediaTrackInfo, TrackSource};
use url::Url;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{Media, MediaId, MediaLink, MediaLinkType, UserId};

use crate::data::DataMedia;

use super::Postgres;

#[derive(Debug, Deserialize)]
pub struct DbMedia {
    id: Uuid,
    filename: String,
    alt: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DbMediaTrack {
    info: DbMediaTrackType,
    height: Option<i64>,
    width: Option<i64>,
    duration: Option<i64>,
    codec: Option<String>,
    language: Option<String>,
    url: String,
    size_type: DbMediaSizeType,
    size: i64,
    source_url: Option<String>,
    mime: String,
    source: DbTrackSource,
}

#[derive(Debug, Deserialize, sqlx::Type)]
#[sqlx(type_name = "media_size_type")]
enum DbMediaSizeType {
    Bytes,
    BytesPerSecond,
}

#[derive(Debug, Deserialize, sqlx::Type)]
#[sqlx(type_name = "media_source")]
enum DbTrackSource {
    Uploaded,
    Downloaded,
    Extracted,
    Generated,
}

#[derive(Debug, Deserialize, sqlx::Type)]
#[sqlx(type_name = "media_track_type")]
enum DbMediaTrackType {
    Video,
    Audio,
    Image,
    Thumbnail,
    TimedText,
    Text,
    Mixed,
    Other,
}

impl From<DbMediaTrack> for MediaTrack {
    fn from(row: DbMediaTrack) -> Self {
        Self {
            // MediaTrackInfo,
            info: match row.info {
                DbMediaTrackType::Video => MediaTrackInfo::Video(types::Video {
                    height: row.height.unwrap().try_into().unwrap(),
                    width: row.width.unwrap().try_into().unwrap(),
                    duration: row.duration.unwrap().try_into().unwrap(),
                    codec: row.codec.unwrap(),
                    language: row.language.map(Into::into),
                }),
                DbMediaTrackType::Audio => MediaTrackInfo::Audio(types::Audio {
                    duration: row.duration.unwrap().try_into().unwrap(),
                    codec: row.codec.unwrap(),
                    language: row.language.map(Into::into),
                }),
                DbMediaTrackType::Image => MediaTrackInfo::Image(types::Image {
                    height: row.height.unwrap().try_into().unwrap(),
                    width: row.width.unwrap().try_into().unwrap(),
                    language: row.language.map(Into::into),
                }),
                DbMediaTrackType::Thumbnail => MediaTrackInfo::Thumbnail(types::Image {
                    height: row.height.unwrap().try_into().unwrap(),
                    width: row.width.unwrap().try_into().unwrap(),
                    language: row.language.map(Into::into),
                }),
                DbMediaTrackType::TimedText => MediaTrackInfo::TimedText(types::TimedText {
                    duration: row.duration.unwrap().try_into().unwrap(),
                    language: row.language.map(Into::into),
                }),
                DbMediaTrackType::Text => MediaTrackInfo::Text(types::Text {
                    language: row.language.map(Into::into),
                }),
                DbMediaTrackType::Mixed => MediaTrackInfo::Mixed(types::Mixed {
                    height: row.height.map(|a| a.try_into().unwrap()),
                    width: row.width.map(|a| a.try_into().unwrap()),
                    duration: row.duration.map(|a| a.try_into().unwrap()),
                    language: row.language.map(Into::into),
                }),
                DbMediaTrackType::Other => MediaTrackInfo::Other,
            },
            url: Url::parse(&row.url).expect("invalid data in db"),
            size: match row.size_type {
                DbMediaSizeType::Bytes => {
                    MediaSize::Bytes(row.size.try_into().expect("invalid size in db"))
                }
                DbMediaSizeType::BytesPerSecond => {
                    MediaSize::BytesPerSecond(row.size.try_into().expect("invalid size in db"))
                }
            },
            mime: row.mime,
            source: match row.source {
                DbTrackSource::Uploaded => TrackSource::Uploaded,
                DbTrackSource::Downloaded => TrackSource::Downloaded {
                    source_url: row
                        .source_url
                        .expect("missing source url")
                        .parse()
                        .expect("invalid source url"),
                },
                DbTrackSource::Extracted => TrackSource::Extracted,
                DbTrackSource::Generated => TrackSource::Generated,
            },
        }
    }
}

impl From<MediaTrack> for DbMediaTrack {
    fn from(value: MediaTrack) -> Self {
        let dims = value.info.dimensions();
        let (size_type, size) = match value.size {
            MediaSize::Bytes(s) => (DbMediaSizeType::Bytes, s.try_into().expect("convert error")),
            MediaSize::BytesPerSecond(s) => (
                DbMediaSizeType::BytesPerSecond,
                s.try_into().expect("convert error"),
            ),
        };
        let (source, source_url) = match value.source {
            TrackSource::Uploaded => (DbTrackSource::Uploaded, None),
            TrackSource::Downloaded { source_url } => {
                (DbTrackSource::Downloaded, Some(source_url.to_string()))
            }
            TrackSource::Extracted => (DbTrackSource::Extracted, None),
            TrackSource::Generated => (DbTrackSource::Generated, None),
        };
        Self {
            info: match value.info {
                MediaTrackInfo::Video(_) => DbMediaTrackType::Video,
                MediaTrackInfo::Audio(_) => DbMediaTrackType::Audio,
                MediaTrackInfo::Image(_) => DbMediaTrackType::Image,
                MediaTrackInfo::Thumbnail(_) => DbMediaTrackType::Thumbnail,
                MediaTrackInfo::TimedText(_) => DbMediaTrackType::TimedText,
                MediaTrackInfo::Text(_) => DbMediaTrackType::Text,
                MediaTrackInfo::Mixed(_) => DbMediaTrackType::Mixed,
                MediaTrackInfo::Other => DbMediaTrackType::Other,
            },
            width: dims.map(|i| i.0.try_into().expect("convert error")),
            height: dims.map(|i| i.1.try_into().expect("convert error")),
            duration: value
                .info
                .duration()
                .map(|i| i.try_into().expect("convert error")),
            codec: value.info.codec().map(|s| s.to_owned()),
            language: value.info.language().map(|i| i.0.to_owned()),
            url: value.url.to_string(),
            size_type,
            size,
            mime: value.mime,
            source,
            source_url,
        }
    }
}

impl DbMedia {
    pub fn upgrade(self, tracks: Vec<DbMediaTrack>) -> Media {
        let mut source = None;
        let mut t2 = vec![];
        for track in tracks {
            let t: MediaTrack = track.into();
            if matches!(
                t.source,
                TrackSource::Downloaded { .. } | TrackSource::Uploaded
            ) {
                if source.is_some() {
                    panic!("already has source");
                }
                source = Some(t);
            } else {
                t2.push(t)
            }
        }
        Media {
            id: self.id.into(),
            filename: self.filename,
            alt: self.alt,
            source: source.expect("missing source"),
            tracks: t2,
        }
    }
}

#[async_trait]
impl DataMedia for Postgres {
    async fn media_insert(&self, user_id: UserId, media: Media) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        query!(
            "
    	    INSERT INTO media (id, user_id, filename, alt)
    	    VALUES ($1, $2, $3, $4)
        ",
            media.id.into_inner(),
            user_id.into_inner(),
            media.filename,
            media.alt,
        )
        .execute(&mut *tx)
        .await?;
        for track in media.all_tracks() {
            let t: DbMediaTrack = track.to_owned().into();
            query!(
                "
    	    INSERT INTO media_track (
                media_id, url, size, size_type, mime,
                source, source_url,
                info, width, height, duration, codec, language
            )
    	    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ",
                media.id.into_inner(),
                t.url,
                t.size,
                t.size_type as _,
                t.mime,
                t.source as _,
                t.source_url,
                t.info as _,
                t.width,
                t.height,
                t.duration,
                t.codec,
                t.language,
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        info!("inserted media");
        Ok(())
    }

    async fn media_select(&self, media_id: MediaId) -> Result<Media> {
        let media = query_as!(
            DbMedia,
            "
    	    SELECT id, filename, alt
    	    FROM media
    	    WHERE id = $1
        ",
            media_id.into_inner(),
        )
        .fetch_one(&self.pool)
        .await?;
        let tracks = query_as!(
            DbMediaTrack,
            r#"
    	    SELECT
        	    url, size_type as "size_type: _", size, mime,
        	    source as "source: _", source_url,
        	    info as "info: _", height, width, duration, codec, language
    	    FROM media_track
    	    WHERE media_id = $1
        "#,
            media_id.into_inner(),
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(media.upgrade(tracks))
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
}
