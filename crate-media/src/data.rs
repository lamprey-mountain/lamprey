use common::v1::types::{EmojiId, Media, MediaId, MediaTrack};
use serde::Deserialize;
use sqlx::{query_scalar, types::JsonValue, Executor, Postgres};

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

pub async fn lookup_media<'e, E>(exec: E, media_id: MediaId) -> Result<Media>
where
    E: Executor<'e, Database = Postgres>,
{
    // TODO: reuse code from crate-backend
    #[derive(Debug, Deserialize)]
    #[serde(tag = "v")]
    pub enum DbMediaData {
        V1(Media),
        // V2(common::v2::types::media::Media),
        #[serde(untagged)]
        Raw(DbMediaRaw),
    }

    #[derive(Debug, Deserialize)]
    pub struct DbMediaRaw {
        id: MediaId,
        filename: String,
        alt: Option<String>,
        tracks: Vec<MediaTrack>,
    }

    impl From<DbMediaData> for Media {
        fn from(value: DbMediaData) -> Self {
            match value {
                DbMediaData::V1(media) => media,
                // DbMediaData::V2(media) => media,
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

    let media: JsonValue = query_scalar!(
        "SELECT data FROM media WHERE id = $1 AND deleted_at IS NULL",
        *media_id
    )
    .fetch_one(exec)
    .await?;
    let media: DbMediaData = serde_json::from_value(media).unwrap();
    Ok(match media {
        DbMediaData::V1(media) => media,
        DbMediaData::Raw(m) => m.into(),
    })
}
