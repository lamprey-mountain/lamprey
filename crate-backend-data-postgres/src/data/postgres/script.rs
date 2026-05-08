use async_trait::async_trait;
use common::v1::types::script::{
    Script, ScriptFormat, ScriptInput, ScriptLocation, ScriptMetadata, ScriptStatus, ScriptVersion,
    ScriptVersionStatus,
};
use common::v1::types::{
    ChannelId, PaginationDirection, PaginationQuery, PaginationResponse, ScriptId, ScriptVerId,
    UserId,
};
use lamprey_backend_core::data::DataScript;
use serde::Deserialize;
use sqlx::{query, query_file_as, query_file_scalar};
use time::PrimitiveDateTime;
use tracing::warn;
use uuid::Uuid;

use crate::{data::postgres::Pagination, error::Result, gen_paginate};

use super::Postgres;

#[derive(Debug, Clone, sqlx::FromRow)]
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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbScriptWithLatestVersion {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub creator_id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub deleted_at: Option<PrimitiveDateTime>,
    pub data: serde_json::Value,
    pub version_id: Uuid,
    pub version_creator_id: Uuid,
    pub version_created_at: PrimitiveDateTime,
    pub version_deleted_at: Option<PrimitiveDateTime>,
    pub version_data: serde_json::Value,
    pub cached_inputs: Option<serde_json::Value>,
    pub version_status: String,
}

impl From<DbScriptWithLatestVersion> for Script {
    fn from(row: DbScriptWithLatestVersion) -> Self {
        let parsed: ScriptData = serde_json::from_value(row.version_data).unwrap_or_default();
        let version_status = match row.version_status.as_str() {
            "Processing" => ScriptVersionStatus::Processing,
            "Valid" => ScriptVersionStatus::Valid,
            "Invalid" => ScriptVersionStatus::Invalid,
            s => {
                warn!("unknown script version status: {s}");
                ScriptVersionStatus::Processing
            }
        };

        let script_status = match version_status {
            ScriptVersionStatus::Processing => ScriptStatus::Processing,
            ScriptVersionStatus::Valid => ScriptStatus::Valid,
            ScriptVersionStatus::Invalid => ScriptStatus::Invalid,
        };

        let inputs: Vec<ScriptInput> = row
            .cached_inputs
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        Script {
            id: row.id.into(),
            channel_id: row.channel_id.into(),
            creator_id: row.creator_id.into(),
            created_at: row.created_at.into(),
            deleted_at: row.deleted_at.map(Into::into),
            latest_version: ScriptVersion {
                version_id: row.version_id.into(),
                created_at: row.version_created_at.into(),
                deleted_at: row.version_deleted_at.map(Into::into),
                format: parsed.format,
                location: parsed.location,
                metadata: parsed.metadata,
                status: version_status,
            },
            status: script_status,
            permissions: vec![],
            inputs,
        }
    }
}

impl From<DbScriptVersion> for ScriptVersion {
    fn from(row: DbScriptVersion) -> Self {
        let parsed: ScriptData = serde_json::from_value(row.data).unwrap_or_default();
        let status = match row.status.as_str() {
            "Processing" => ScriptVersionStatus::Processing,
            "Valid" => ScriptVersionStatus::Valid,
            "Invalid" => ScriptVersionStatus::Invalid,
            s => {
                warn!("unknown script version status: {s}");
                ScriptVersionStatus::Processing
            }
        };

        ScriptVersion {
            version_id: row.version_id.into(),
            created_at: row.created_at.into(),
            deleted_at: row.deleted_at.map(Into::into),
            format: parsed.format,
            location: parsed.location,
            metadata: parsed.metadata,
            status,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
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
            metadata: ScriptMetadata::default(),
        }
    }
}

#[async_trait]
impl DataScript for Postgres {
    async fn script_create(&mut self, script: &Script) -> Result<()> {
        let mut conn = self.acquire().await?;

        let data = serde_json::json!({
            "format": script.latest_version.format,
            "location": script.latest_version.location,
            "metadata": script.latest_version.metadata,
        });

        query!(
            r#"
            INSERT INTO script (id, channel_id, creator_id, created_at, deleted_at, data)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP, NULL, $4)
            "#,
            *script.id,
            *script.channel_id,
            *script.creator_id,
            data
        )
        .execute(conn.ext())
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

        query!(
            r#"
            INSERT INTO script_version (version_id, script_id, channel_id, creator_id, created_at, deleted_at, data, cached_inputs, status)
            VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, NULL, $5, $6, 'Processing')
            "#,
            *version_id,
            *script_id,
            *channel_id,
            *creator_id,
            data,
            cached_inputs
        )
        .execute(conn.ext())
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
        let row = query_file_as!(DbScriptWithLatestVersion, "sql/script_get.sql", *script_id)
            .fetch_optional(conn.ext())
            .await?;

        Ok(row.map(Script::from))
    }

    async fn script_list_by_channel(
        &mut self,
        channel_id: ChannelId,
        pagination: PaginationQuery<ScriptId>,
    ) -> Result<PaginationResponse<Script>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
            query_file_as!(
                DbScriptWithLatestVersion,
                "sql/script_list_by_channel.sql",
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/script_count_by_channel.sql", *channel_id),
            |row: DbScriptWithLatestVersion| Script::from(row),
            |s: &Script| s.id.to_string()
        )
    }

    async fn script_version_list_by_script(
        &mut self,
        channel_id: ChannelId,
        script_id: ScriptId,
        pagination: PaginationQuery<ScriptVerId>,
    ) -> Result<PaginationResponse<ScriptVersion>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
            query_file_as!(
                DbScriptVersion,
                "sql/script_version_paginate.sql",
                *script_id,
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/script_version_count.sql", *script_id, *channel_id),
            |v: DbScriptVersion| ScriptVersion::from(v),
            |v: &ScriptVersion| v.version_id.to_string()
        )
    }
}
