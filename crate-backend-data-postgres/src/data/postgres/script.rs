// TODO: create enum variants in db

use async_trait::async_trait;
use common::v1::types::script::{
    Run, RunInput, RunLogEntry, RunLogLevel, RunStatus, Script, ScriptFormat, ScriptInput,
    ScriptLocation, ScriptMetadata, ScriptStatus, ScriptVersion, ScriptVersionStatus,
};
use common::v1::types::{
    ChannelId, PaginationDirection, PaginationQuery, PaginationResponse, RunId, ScriptId,
    ScriptVerId, UserId,
};
use lamprey_backend_core::data::DataScript;
use serde::Deserialize;
use sqlx::{query, query_file, query_file_as, query_file_scalar};
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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbRun {
    pub id: Uuid,
    pub script_id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub stopped_at: Option<PrimitiveDateTime>,
    pub status: i16,
    pub input: serde_json::Value,
}

impl From<DbRun> for Run {
    fn from(row: DbRun) -> Self {
        let status = match row.status {
            0 => RunStatus::Creating,
            1 => RunStatus::Active,
            2 => RunStatus::Sleeping,
            3 => RunStatus::Waking,
            4 => RunStatus::Exited,
            5 => RunStatus::Borked,
            6 => RunStatus::Crashed,
            7 => RunStatus::Stopped,
            _ => RunStatus::Crashed,
        };
        let input = serde_json::from_value(row.input).unwrap_or(RunInput::Extraction);
        Run {
            id: row.id.into(),
            script_id: row.script_id.into(),
            created_at: row.created_at.into(),
            stopped_at: row.stopped_at.map(Into::into),
            status,
            input,
        }
    }
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

    async fn script_version_update_status(
        &mut self,
        _script_id: ScriptId,
        version_id: ScriptVerId,
        status: ScriptVersionStatus,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        let status_str = match status {
            ScriptVersionStatus::Processing => "Processing",
            ScriptVersionStatus::Valid => "Valid",
            ScriptVersionStatus::Invalid => "Invalid",
        };

        query!(
            "UPDATE script_version SET status = $2 WHERE version_id = $1",
            *version_id,
            status_str
        )
        .execute(conn.ext())
        .await?;

        Ok(())
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

    async fn script_version_get(
        &mut self,
        script_id: ScriptId,
        channel_id: ChannelId,
        version_id: ScriptVerId,
    ) -> Result<Option<ScriptVersion>> {
        let mut conn = self.acquire().await?;
        let row = query_file_as!(
            DbScriptVersion,
            "sql/script_version_get.sql",
            *script_id,
            *channel_id,
            *version_id
        )
        .fetch_optional(conn.ext())
        .await?;

        Ok(row.map(ScriptVersion::from))
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

    async fn script_log_insert(&mut self, run_id: RunId, entry: &RunLogEntry) -> Result<()> {
        let mut conn = self.acquire().await?;

        let level_int = match entry.level {
            RunLogLevel::Trace => 0,
            RunLogLevel::Debug => 1,
            RunLogLevel::Info => 2,
            RunLogLevel::Warning => 3,
            RunLogLevel::Error => 4,
        };

        let source_json = serde_json::to_value(&entry.source)?;
        let attrs = serde_json::to_value(&entry.attributes)?;

        query!(
            r#"
            INSERT INTO script_log (run_id, line_id, created_at, level, source, content, attributes)
            VALUES ($1, (SELECT COALESCE(MAX(line_id), -1) + 1 FROM script_log WHERE run_id = $1), CURRENT_TIMESTAMP, $2, $3, $4, $5)
            "#,
            *run_id,
            level_int,
            &source_json,
            &entry.content,
            attrs,
        )
        .execute(conn.ext())
        .await?;

        Ok(())
    }

    async fn script_log_list(
        &mut self,
        run_id: RunId,
        pagination: PaginationQuery<u64>,
    ) -> Result<PaginationResponse<RunLogEntry>> {
        let p: Pagination<_> = pagination.try_into()?;
        let run_id_uuid = *run_id;

        gen_paginate!(
            p,
            self,
            query_file!(
                "sql/script_log_list.sql",
                run_id_uuid,
                p.after as i64,
                p.before.min(i64::MAX as u64) as i64,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/script_log_count.sql", run_id_uuid),
            |row| {
                let level_int: i16 = row.level;
                let level = match level_int {
                    0 => RunLogLevel::Trace,
                    1 => RunLogLevel::Debug,
                    2 => RunLogLevel::Info,
                    3 => RunLogLevel::Warning,
                    4 => RunLogLevel::Error,
                    _ => RunLogLevel::Info,
                };
                RunLogEntry {
                    id: row.line_id as u64,
                    created_at: row.created_at.into(),
                    level,
                    source: serde_json::from_value(row.source).expect("invalid data in db"),
                    content: row.content,
                    attributes: serde_json::from_value(row.attributes).unwrap_or_default(),
                }
            },
            |v: &RunLogEntry| v.id.to_string()
        )
    }

    async fn script_run_create(&mut self, run: &Run) -> Result<()> {
        let mut conn = self.acquire().await?;
        let status_int = match run.status {
            RunStatus::Creating => 0,
            RunStatus::Active => 1,
            RunStatus::Sleeping => 2,
            RunStatus::Waking => 3,
            RunStatus::Exited => 4,
            RunStatus::Borked => 5,
            RunStatus::Crashed => 6,
            RunStatus::Stopped => 7,
        };

        let input_json = serde_json::to_value(&run.input).unwrap_or(serde_json::Value::Null);

        query!(
            r#"
            INSERT INTO script_run (id, script_id, created_at, stopped_at, status, input)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            *run.id,
            *run.script_id,
            PrimitiveDateTime::from(run.created_at),
            run.stopped_at.map(PrimitiveDateTime::from),
            status_int,
            input_json
        )
        .execute(conn.ext())
        .await?;

        Ok(())
    }

    async fn script_run_get(&mut self, run_id: RunId) -> Result<Option<Run>> {
        let mut conn = self.acquire().await?;
        let row = query_file_as!(DbRun, "sql/script_run_get.sql", *run_id)
            .fetch_optional(conn.ext())
            .await?;

        Ok(row.map(Into::into))
    }

    async fn script_run_list(
        &mut self,
        script_id: ScriptId,
        pagination: PaginationQuery<RunId>,
    ) -> Result<PaginationResponse<Run>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
            query_file_as!(
                DbRun,
                "sql/script_run_list.sql",
                *script_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/script_run_count.sql", *script_id),
            |row: DbRun| Run::from(row),
            |r: &Run| r.id.to_string()
        )
    }

    async fn script_run_update_status(&mut self, run_id: RunId, status: RunStatus) -> Result<()> {
        let mut conn = self.acquire().await?;
        let status_int = match status {
            RunStatus::Creating => 0,
            RunStatus::Active => 1,
            RunStatus::Sleeping => 2,
            RunStatus::Waking => 3,
            RunStatus::Exited => 4,
            RunStatus::Borked => 5,
            RunStatus::Crashed => 6,
            RunStatus::Stopped => 7,
        };

        let is_terminal = matches!(
            status,
            RunStatus::Exited | RunStatus::Borked | RunStatus::Crashed | RunStatus::Stopped
        );

        if is_terminal {
            query!(
                "UPDATE script_run SET status = $2, stopped_at = CURRENT_TIMESTAMP WHERE id = $1",
                *run_id,
                status_int
            )
            .execute(conn.ext())
            .await?;
        } else {
            query!(
                "UPDATE script_run SET status = $2 WHERE id = $1",
                *run_id,
                status_int
            )
            .execute(conn.ext())
            .await?;
        }

        Ok(())
    }

    async fn script_run_stop(&mut self, run_id: RunId) -> Result<()> {
        self.script_run_update_status(run_id, RunStatus::Stopped)
            .await
    }
}
