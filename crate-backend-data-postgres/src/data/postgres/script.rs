use async_trait::async_trait;
use common::v1::types::script::{Script, ScriptStatus, ScriptVersion, ScriptVersionStatus};
use common::v1::types::{
    script::{ScriptFormat, ScriptLocation, ScriptMetadata},
    ChannelId, PaginationDirection, PaginationKey, PaginationQuery, PaginationResponse, ScriptId,
    ScriptVerId, UserId,
};
use lamprey_backend_core::data::DataScript;
use serde::Deserialize;
use sqlx::{query, query_as, query_scalar};
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{data::postgres::Pagination, error::Result, gen_paginate};

use super::Postgres;

pub struct DbScript {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub creator_id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub deleted_at: Option<PrimitiveDateTime>,
    pub data: serde_json::Value,
}

pub struct DbScriptVersion {
    pub version_id: Uuid,
    pub script_id: Uuid,
    pub channel_id: Uuid,
    pub creator_id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub deleted_at: Option<PrimitiveDateTime>,
    pub data: serde_json::Value,
    pub cached_inputs: Option<serde_json::Value>,
    pub status: String,
}

impl From<DbScript> for Script {
    fn from(val: DbScript) -> Self {
        let data: serde_json::Value = val.data;
        let parsed: ScriptData = serde_json::from_value(data).unwrap_or_default();
        Self {
            id: val.id.into(),
            channel_id: val.channel_id.into(),
            creator_id: val.creator_id.into(),
            created_at: val.created_at.into(),
            deleted_at: val.deleted_at.map(Into::into),
            latest_version: ScriptVersion {
                version_id: ScriptVerId::new(),
                created_at: val.created_at.into(),
                deleted_at: val.deleted_at.map(Into::into),
                format: parsed.format,
                location: parsed.location,
                metadata: parsed.metadata,
                status: ScriptVersionStatus::Processing,
            },
            status: ScriptStatus::Empty,
            permissions: vec![],
            inputs: vec![],
        }
    }
}

impl From<DbScriptVersion> for ScriptVersion {
    fn from(val: DbScriptVersion) -> Self {
        let data: serde_json::Value = val.data;
        let parsed: ScriptData = serde_json::from_value(data).unwrap_or_default();
        let status = match val.status.as_str() {
            "Processing" => ScriptVersionStatus::Processing,
            "Valid" => ScriptVersionStatus::Valid,
            "Invalid" => ScriptVersionStatus::Invalid,
            _ => ScriptVersionStatus::Processing,
        };
        Self {
            version_id: val.version_id.into(),
            created_at: val.created_at.into(),
            deleted_at: val.deleted_at.map(Into::into),
            format: parsed.format,
            location: parsed.location,
            metadata: parsed.metadata,
            status,
        }
    }
}

struct ScriptData {
    format: ScriptFormat,
    location: ScriptLocation,
    metadata: ScriptMetadata,
}

impl Default for ScriptData {
    fn default() -> Self {
        Self {
            format: ScriptFormat::Javascript,
            location: ScriptLocation::Local {
                path: String::new(),
            },
            metadata: ScriptMetadata {
                name: String::new(),
                description: None,
                homepage_url: String::new(),
                authors: Vec::new(),
                version: String::new(),
                license: String::new(),
            },
        }
    }
}

impl<'de> Deserialize<'de> for ScriptData {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawData {
            format: Option<ScriptFormat>,
            location: Option<ScriptLocation>,
            metadata: Option<ScriptMetadata>,
        }
        let raw = RawData::deserialize(deserializer)?;
        Ok(Self {
            format: raw.format.unwrap_or(ScriptFormat::Javascript),
            location: raw.location.unwrap_or(ScriptLocation::Local {
                path: String::new(),
            }),
            metadata: raw.metadata.unwrap_or_else(|| ScriptMetadata {
                name: String::new(),
                description: None,
                homepage_url: String::new(),
                authors: Vec::new(),
                version: String::new(),
                license: String::new(),
            }),
        })
    }
}

#[async_trait]
impl DataScript for Postgres {
    /// create a new script and script version
    async fn script_create(&mut self, script: &Script) -> Result<()> {
        let mut conn = self.acquire().await?;

        let data = serde_json::json!({
            "format": script.latest_version.format,
            "location": script.latest_version.location,
            "metadata": script.latest_version.metadata,
        });

        query_as!(
            DbScript,
            r#"
            INSERT INTO script (id, channel_id, creator_id, created_at, deleted_at, data)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP, NULL, $4)
            RETURNING id, channel_id, creator_id, created_at, deleted_at, data
            "#,
            *script.id,
            *script.channel_id,
            *script.creator_id,
            data
        )
        .fetch_one(conn.ext())
        .await?;

        Ok(())
    }

    async fn script_update(
        &mut self,
        script_id: ScriptId,
        format: ScriptFormat,
        location: ScriptLocation,
        metadata: ScriptMetadata,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;

        let data = serde_json::json!({
            "format": format,
            "location": location,
            "metadata": metadata,
        });

        query!(
            "UPDATE script SET data = $2 WHERE id = $1",
            *script_id,
            data
        )
        .execute(conn.ext())
        .await?;

        Ok(())
    }

    async fn script_delete(&mut self, script_id: ScriptId) -> Result<()> {
        let mut conn = self.acquire().await?;

        query!(
            "UPDATE script SET deleted_at = now() WHERE id = $1",
            *script_id
        )
        .execute(conn.ext())
        .await?;

        Ok(())
    }

    async fn script_version_create(
        &mut self,
        script_id: ScriptId,
        channel_id: ChannelId,
        creator_id: UserId,
        format: ScriptFormat,
        location: ScriptLocation,
        metadata: ScriptMetadata,
        cached_inputs: Option<serde_json::Value>,
    ) -> Result<ScriptVerId> {
        let version_id = ScriptVerId::new();
        let mut conn = self.acquire().await?;

        let data = serde_json::json!({
            "format": format,
            "location": location,
            "metadata": metadata,
        });

        query_as!(
            DbScriptVersion,
            r#"
            INSERT INTO script_version (version_id, script_id, channel_id, creator_id, created_at, deleted_at, data, cached_inputs, status)
            VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, NULL, $5, $6, 'Processing')
            RETURNING version_id, script_id, channel_id, creator_id, created_at, deleted_at, data, cached_inputs, status
            "#,
            *version_id,
            *script_id,
            *channel_id,
            *creator_id,
            data,
            cached_inputs
        )
        .fetch_one(conn.ext())
        .await?;

        Ok(version_id)
    }

    async fn script_version_delete(
        &mut self,
        script_id: ScriptId,
        version_id: ScriptVerId,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;

        query!(
            "UPDATE script_version SET deleted_at = now() WHERE version_id = $1 AND script_id = $2",
            *version_id,
            *script_id
        )
        .execute(conn.ext())
        .await?;

        Ok(())
    }

    async fn script_get(&mut self, script_id: ScriptId) -> Result<Option<Script>> {
        let mut conn = self.acquire().await?;

        let script = query_as!(
            DbScript,
            r#"
            SELECT id, channel_id, creator_id, created_at, deleted_at, data
            FROM script
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            *script_id
        )
        .fetch_optional(conn.ext())
        .await?;

        Ok(script.map(|s| s.into()))
    }

    async fn script_list_by_channel(
        &mut self,
        channel_id: ChannelId,
        query: PaginationQuery<ScriptId>,
    ) -> Result<PaginationResponse<Script>> {
        let dir = query.dir.unwrap_or_default();
        let after = match dir {
            PaginationDirection::F => query.from.clone(),
            _ => query.to.clone(),
        };
        let after = after.unwrap_or(PaginationKey::min());
        let before = match dir {
            PaginationDirection::F => query.to,
            _ => query.from,
        };
        let before = before.unwrap_or(PaginationKey::max());
        let p: Pagination<_> = Pagination {
            before,
            after,
            dir,
            limit: query.limit.unwrap_or(10),
        };

        gen_paginate!(
            p,
            self,
            query_as!(
                DbScript,
                r#"
                SELECT id, channel_id, creator_id, created_at, deleted_at, data
                FROM script
                WHERE channel_id = $1 AND deleted_at IS NULL AND id > $2 AND id < $3
                ORDER BY id LIMIT $4
                "#,
                *channel_id,
                *p.after,
                *p.before,
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM script WHERE channel_id = $1 AND deleted_at IS NULL",
                *channel_id
            ),
            |s: &Script| s.id.to_string()
        )
    }

    async fn script_version_list_by_script(
        &mut self,
        channel_id: ChannelId,
        script_id: ScriptId,
        query: PaginationQuery<ScriptVerId>,
    ) -> Result<PaginationResponse<ScriptVersion>> {
        let dir = query.dir.unwrap_or_default();
        let after = match dir {
            PaginationDirection::F => query.from.clone(),
            _ => query.to.clone(),
        };
        let after = after.unwrap_or(PaginationKey::min());
        let before = match dir {
            PaginationDirection::F => query.to,
            _ => query.from,
        };
        let before = before.unwrap_or(PaginationKey::max());
        let p: Pagination<_> = Pagination {
            before,
            after,
            dir,
            limit: query.limit.unwrap_or(10),
        };

        gen_paginate!(
            p, self,
            query_as!(
                DbScriptVersion,
                r#"
                SELECT version_id, script_id, channel_id, creator_id, created_at, deleted_at, data, cached_inputs, status
                FROM script_version
                WHERE script_id = $1 AND channel_id = $2 AND deleted_at IS NULL AND version_id > $3 AND version_id < $4
                ORDER BY version_id LIMIT $5
                "#,
                *script_id,
                *channel_id,
                *p.after,
                *p.before,
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM script_version WHERE script_id = $1 AND channel_id = $2 AND deleted_at IS NULL",
                *script_id,
                *channel_id
            ),
            |v: &ScriptVersion| v.version_id.to_string()
        )
    }
}
