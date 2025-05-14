use std::time::Duration;

use async_trait::async_trait;
use common::v1::types::{misc::Color, Embed, EmbedId, MessageVerId, UserId};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{query, query_as};
use tracing::debug;
use url::Url;
use uuid::Uuid;

use super::{util::media_from_db, Postgres};

use crate::{data::DataEmbed, Result};

#[derive(Debug, Deserialize)]
pub struct DbEmbed {
    pub id: Uuid,
    pub url: String,
    pub canonical_url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub media: Option<Value>,
    pub media_is_thumbnail: Option<bool>,
    pub author_url: Option<String>,
    pub author_name: Option<String>,
    pub author_avatar: Option<Value>,
    pub site_name: Option<String>,
    pub site_avatar: Option<Value>,
}

impl From<DbEmbed> for Embed {
    fn from(row: DbEmbed) -> Self {
        Embed {
            id: row.id.into(),
            url: row.url.parse().expect("invalid data in db"),
            canonical_url: row
                .canonical_url
                .map(|i| i.parse().expect("invalid data in db")),
            title: row.title,
            description: row.description,
            color: row.color.map(Color::from_hex_string),
            media: row.media.map(media_from_db),
            media_is_thumbnail: row.media_is_thumbnail.expect("invalid data in db"),
            author_url: row
                .author_url
                .map(|i| i.parse().expect("invalid data in db")),
            author_name: row.author_name,
            author_avatar: row.author_avatar.map(media_from_db),
            site_name: row.site_name,
            site_avatar: row.site_avatar.map(media_from_db),
        }
    }
}

#[async_trait]
impl DataEmbed for Postgres {
    async fn embed_insert(&self, user_id: UserId, embed: Embed) -> Result<()> {
        query!(
            r#"
            INSERT INTO url_embed (
                id,
                url,
                canonical_url,
                title,
                description,
                color,
                media,
                media_is_thumbnail,
                author_url,
                author_name,
                author_avatar,
                site_name,
                site_avatar,
                user_id
            )
    	    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
            embed.id.into_inner(),
            embed.url.to_string(),
            embed.canonical_url.map(|u| u.to_string()),
            embed.title,
            embed.description,
            embed.color.map(|c| c.as_ref().to_string()),
            embed.media.map(|m| m.id.into_inner()),
            embed.media_is_thumbnail,
            embed.author_url.map(|u| u.to_string()),
            embed.author_name,
            embed.author_avatar.map(|m| m.id.into_inner()),
            embed.site_name,
            embed.site_avatar.map(|m| m.id.into_inner()),
            user_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        debug!("inserted embed");
        Ok(())
    }

    async fn embed_find(&self, url: Url, max_age: Duration) -> Result<Option<Embed>> {
        let min_ts = time::OffsetDateTime::now_utc() - max_age;
        let min_ts = time::PrimitiveDateTime::new(min_ts.date(), min_ts.time());
        let row = query_as!(
            DbEmbed,
            r#"
            SELECT
                u.id,
                u.url,
                u.canonical_url,
                u.title,
                u.description,
                u.color,
                row_to_json(m) as media,
                u.media_is_thumbnail,
                u.author_url,
                u.author_name,
                row_to_json(a) as author_avatar,
                u.site_name,
                row_to_json(s) as site_avatar
            FROM url_embed u
            JOIN media_json m ON m.id = u.media
            JOIN media_json a ON a.id = u.author_avatar
            JOIN media_json s ON s.id = u.site_avatar
            WHERE u.url = $1 AND u.created_at > $2
            "#,
            url.to_string(),
            min_ts,
        )
        .fetch_optional(&self.pool)
        .await?;
        let embed = row.map(|r| r.into());
        if embed.is_some() {
            debug!("found embed url={url}");
        } else {
            debug!("found no embed url={url}");
        }
        Ok(embed)
    }

    async fn embed_link(
        &self,
        version_id: MessageVerId,
        embed_id: EmbedId,
        ordering: u32,
    ) -> Result<()> {
        query!(
            r#"
            INSERT INTO url_embed_message (version_id, embed_id, ordering)
    	    VALUES ($1, $2, $3)
        "#,
            version_id.into_inner(),
            embed_id.into_inner(),
            ordering as i64,
        )
        .execute(&self.pool)
        .await?;
        debug!("linked embed version_id={version_id} embed_id={embed_id}");
        Ok(())
    }
}
