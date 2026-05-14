// TODO: create enum variants in db

use async_trait::async_trait;
use common::v1::types::redex::{
    Eval, EvalInputSummary, EvalLogEntry, EvalLogLevel, EvalStatus, Redex, RedexFormat,
    RedexHandler, RedexLocation, RedexMetadata, RedexStatus, RedexVersion, RedexVersionStatus,
};
use common::v1::types::{
    ChannelId, EvalId, PaginationDirection, PaginationQuery, PaginationResponse, RedexId,
    RedexVerId, UserId,
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
    pub redex_version_id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub stopped_at: Option<PrimitiveDateTime>,
    pub status: i16,
    pub input: serde_json::Value,
}

impl From<DbRun> for Eval {
    fn from(row: DbRun) -> Self {
        let status = match row.status {
            0 => EvalStatus::Creating,
            1 => EvalStatus::Active,
            2 => EvalStatus::Sleeping,
            3 => EvalStatus::Waking,
            4 => EvalStatus::Exited,
            5 => EvalStatus::Borked,
            6 => EvalStatus::Crashed,
            7 => EvalStatus::Stopped,
            _ => EvalStatus::Crashed,
        };
        let input = serde_json::from_value(row.input).unwrap_or(EvalInputSummary::Extraction);
        Eval {
            id: row.id.into(),
            redex_id: row.script_id.into(),
            redex_version_id: row.redex_version_id.into(),
            created_at: row.created_at.into(),
            stopped_at: row.stopped_at.map(Into::into),
            status,
            input,
        }
    }
}

impl From<DbScriptWithLatestVersion> for Redex {
    fn from(row: DbScriptWithLatestVersion) -> Self {
        let parsed: ScriptData = serde_json::from_value(row.version_data).unwrap_or_default();
        let version_status = match row.version_status.as_str() {
            "Processing" => RedexVersionStatus::Processing,
            "Valid" => RedexVersionStatus::Valid,
            "Invalid" => RedexVersionStatus::Invalid,
            s => {
                warn!("unknown redex version status: {s}");
                RedexVersionStatus::Processing
            }
        };

        let script_status = match version_status {
            RedexVersionStatus::Processing => RedexStatus::Processing,
            RedexVersionStatus::Valid => RedexStatus::Valid,
            RedexVersionStatus::Invalid => RedexStatus::Invalid,
        };

        let inputs: Vec<RedexHandler> = row
            .cached_inputs
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        Redex {
            id: row.id.into(),
            channel_id: row.channel_id.into(),
            creator_id: row.creator_id.into(),
            created_at: row.created_at.into(),
            deleted_at: row.deleted_at.map(Into::into),
            latest_version: RedexVersion {
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
            handlers: inputs,
        }
    }
}

impl From<DbScriptVersion> for RedexVersion {
    fn from(row: DbScriptVersion) -> Self {
        let parsed: ScriptData = serde_json::from_value(row.data).unwrap_or_default();
        let status = match row.status.as_str() {
            "Processing" => RedexVersionStatus::Processing,
            "Valid" => RedexVersionStatus::Valid,
            "Invalid" => RedexVersionStatus::Invalid,
            s => {
                warn!("unknown redex version status: {s}");
                RedexVersionStatus::Processing
            }
        };

        RedexVersion {
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
    format: RedexFormat,
    location: RedexLocation,
    metadata: RedexMetadata,
}

impl Default for ScriptData {
    fn default() -> Self {
        Self {
            format: RedexFormat::Javascript,
            location: RedexLocation::Local {
                path: String::new(),
            },
            metadata: RedexMetadata::new("unnamed".to_owned()),
        }
    }
}

#[async_trait]
impl DataScript for Postgres {
    async fn script_create(&mut self, script: &Redex) -> Result<()> {
        let mut conn = self.acquire().await?;

        let data = serde_json::json!({
            "format": script.latest_version.format,
            "location": script.latest_version.location,
            "metadata": script.latest_version.metadata,
        });

        query!(
            r#"
            INSERT INTO redex (id, channel_id, creator_id, created_at, deleted_at, data)
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
        script_id: RedexId,
        format: RedexFormat,
        location: RedexLocation,
        metadata: RedexMetadata,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;

        let data = serde_json::json!({
            "format": format,
            "location": location,
            "metadata": metadata,
        });

        query!("UPDATE redex SET data = $2 WHERE id = $1", *script_id, data)
            .execute(conn.ext())
            .await?;

        Ok(())
    }

    async fn script_delete(&mut self, script_id: RedexId) -> Result<()> {
        let mut conn = self.acquire().await?;

        query!(
            "UPDATE redex SET deleted_at = now() WHERE id = $1",
            *script_id
        )
        .execute(conn.ext())
        .await?;

        Ok(())
    }

    async fn script_version_create(
        &mut self,
        script_id: RedexId,
        channel_id: ChannelId,
        creator_id: UserId,
        format: RedexFormat,
        location: RedexLocation,
        metadata: RedexMetadata,
        cached_inputs: Option<serde_json::Value>,
    ) -> Result<RedexVerId> {
        let version_id = RedexVerId::new();
        let mut conn = self.acquire().await?;

        let data = serde_json::json!({
            "format": format,
            "location": location,
            "metadata": metadata,
        });

        query!(
            r#"
            INSERT INTO redex_version (version_id, script_id, channel_id, creator_id, created_at, deleted_at, data, cached_inputs, status)
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
        _script_id: RedexId,
        version_id: RedexVerId,
        status: RedexVersionStatus,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        let status_str = match status {
            RedexVersionStatus::Processing => "Processing",
            RedexVersionStatus::Valid => "Valid",
            RedexVersionStatus::Invalid => "Invalid",
        };

        query!(
            "UPDATE redex_version SET status = $2 WHERE version_id = $1",
            *version_id,
            status_str
        )
        .execute(conn.ext())
        .await?;

        Ok(())
    }

    async fn script_version_delete(
        &mut self,
        script_id: RedexId,
        version_id: RedexVerId,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;

        query!(
            "UPDATE redex_version SET deleted_at = now() WHERE version_id = $1 AND script_id = $2",
            *version_id,
            *script_id
        )
        .execute(conn.ext())
        .await?;

        Ok(())
    }

    async fn script_version_get(
        &mut self,
        script_id: RedexId,
        channel_id: ChannelId,
        version_id: RedexVerId,
    ) -> Result<Option<RedexVersion>> {
        let mut conn = self.acquire().await?;
        let row = query_file_as!(
            DbScriptVersion,
            "sql/redex_version_get.sql",
            *script_id,
            *channel_id,
            *version_id
        )
        .fetch_optional(conn.ext())
        .await?;

        Ok(row.map(RedexVersion::from))
    }

    async fn script_get(&mut self, script_id: RedexId) -> Result<Option<Redex>> {
        let mut conn = self.acquire().await?;
        let row = query_file_as!(DbScriptWithLatestVersion, "sql/redex_get.sql", *script_id)
            .fetch_optional(conn.ext())
            .await?;

        Ok(row.map(Redex::from))
    }

    async fn script_list_by_channel(
        &mut self,
        channel_id: ChannelId,
        pagination: PaginationQuery<RedexId>,
    ) -> Result<PaginationResponse<Redex>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
            query_file_as!(
                DbScriptWithLatestVersion,
                "sql/redex_list_by_channel.sql",
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/redex_count_by_channel.sql", *channel_id),
            |row: DbScriptWithLatestVersion| Redex::from(row),
            |s: &Redex| s.id.to_string()
        )
    }

    async fn script_version_list_by_script(
        &mut self,
        channel_id: ChannelId,
        script_id: RedexId,
        pagination: PaginationQuery<RedexVerId>,
    ) -> Result<PaginationResponse<RedexVersion>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
            query_file_as!(
                DbScriptVersion,
                "sql/redex_version_paginate.sql",
                *script_id,
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/redex_version_count.sql", *script_id, *channel_id),
            |v: DbScriptVersion| RedexVersion::from(v),
            |v: &RedexVersion| v.version_id.to_string()
        )
    }

    async fn script_log_insert(&mut self, run_id: EvalId, entry: &EvalLogEntry) -> Result<()> {
        let mut conn = self.acquire().await?;

        let level_int = match entry.level {
            EvalLogLevel::Trace => 0,
            EvalLogLevel::Debug => 1,
            EvalLogLevel::Info => 2,
            EvalLogLevel::Warning => 3,
            EvalLogLevel::Error => 4,
        };

        let source_json = serde_json::to_value(&entry.source)?;
        let attrs = serde_json::to_value(&entry.attributes)?;

        query!(
            r#"
            INSERT INTO redex_log (run_id, line_id, created_at, level, source, content, attributes)
            VALUES ($1, (SELECT COALESCE(MAX(line_id), -1) + 1 FROM redex_log WHERE run_id = $1), CURRENT_TIMESTAMP, $2, $3, $4, $5)
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
        run_id: EvalId,
        pagination: PaginationQuery<u64>,
    ) -> Result<PaginationResponse<EvalLogEntry>> {
        let p: Pagination<_> = pagination.try_into()?;
        let run_id_uuid = *run_id;

        gen_paginate!(
            p,
            self,
            query_file!(
                "sql/redex_log_list.sql",
                run_id_uuid,
                p.after as i64,
                p.before.min(i64::MAX as u64) as i64,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/redex_log_count.sql", run_id_uuid),
            |row| {
                let level_int: i16 = row.level;
                let level = match level_int {
                    0 => EvalLogLevel::Trace,
                    1 => EvalLogLevel::Debug,
                    2 => EvalLogLevel::Info,
                    3 => EvalLogLevel::Warning,
                    4 => EvalLogLevel::Error,
                    _ => EvalLogLevel::Info,
                };
                EvalLogEntry {
                    id: row.line_id as u64,
                    created_at: row.created_at.into(),
                    level,
                    source: serde_json::from_value(row.source).expect("invalid data in db"),
                    content: row.content,
                    attributes: serde_json::from_value(row.attributes).unwrap_or_default(),
                }
            },
            |v: &EvalLogEntry| v.id.to_string()
        )
    }

    async fn script_run_create(&mut self, run: &Eval) -> Result<()> {
        let mut conn = self.acquire().await?;
        let status_int = match run.status {
            EvalStatus::Creating => 0,
            EvalStatus::Active => 1,
            EvalStatus::Sleeping => 2,
            EvalStatus::Waking => 3,
            EvalStatus::Exited => 4,
            EvalStatus::Borked => 5,
            EvalStatus::Crashed => 6,
            EvalStatus::Stopped => 7,
        };

        let input_json = serde_json::to_value(&run.input).unwrap_or(serde_json::Value::Null);

        query!(
            r#"
            INSERT INTO redex_eval (id, script_id, redex_version_id, created_at, stopped_at, status, input)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            *run.id,
            *run.redex_id,
            *run.redex_version_id,
            PrimitiveDateTime::from(run.created_at),
            run.stopped_at.map(PrimitiveDateTime::from),
            status_int,
            input_json
        )
        .execute(conn.ext())
        .await?;

        Ok(())
    }

    async fn script_run_get(&mut self, run_id: EvalId) -> Result<Option<Eval>> {
        let mut conn = self.acquire().await?;
        let row = query_file_as!(DbRun, "sql/redex_run_get.sql", *run_id)
            .fetch_optional(conn.ext())
            .await?;

        Ok(row.map(Into::into))
    }

    async fn script_run_list(
        &mut self,
        script_id: RedexId,
        pagination: PaginationQuery<EvalId>,
    ) -> Result<PaginationResponse<Eval>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
            query_file_as!(
                DbRun,
                "sql/redex_run_list.sql",
                *script_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/redex_run_count.sql", *script_id),
            |row: DbRun| Eval::from(row),
            |r: &Eval| r.id.to_string()
        )
    }

    async fn script_run_update_status(&mut self, run_id: EvalId, status: EvalStatus) -> Result<()> {
        let mut conn = self.acquire().await?;
        let status_int = match status {
            EvalStatus::Creating => 0,
            EvalStatus::Active => 1,
            EvalStatus::Sleeping => 2,
            EvalStatus::Waking => 3,
            EvalStatus::Exited => 4,
            EvalStatus::Borked => 5,
            EvalStatus::Crashed => 6,
            EvalStatus::Stopped => 7,
        };

        let is_terminal = matches!(
            status,
            EvalStatus::Exited | EvalStatus::Borked | EvalStatus::Crashed | EvalStatus::Stopped
        );

        if is_terminal {
            query!(
                "UPDATE redex_eval SET status = $2, stopped_at = CURRENT_TIMESTAMP WHERE id = $1",
                *run_id,
                status_int
            )
            .execute(conn.ext())
            .await?;
        } else {
            query!(
                "UPDATE redex_eval SET status = $2 WHERE id = $1",
                *run_id,
                status_int
            )
            .execute(conn.ext())
            .await?;
        }

        Ok(())
    }

    async fn script_run_stop(&mut self, run_id: EvalId) -> Result<()> {
        self.script_run_update_status(run_id, EvalStatus::Stopped)
            .await
    }
}
