use crate::data::postgres::{Pagination, Postgres};
use crate::data::DataDocument;
use crate::error::{Error, Result};
use crate::services::documents::EditContextId;
use crate::types::{DehydratedDocument, PaginationDirection};
use async_trait::async_trait;
use common::v1::types::document::{
    DocumentBranch, DocumentBranchCreate, DocumentBranchListParams, DocumentBranchPatch,
    DocumentBranchState, DocumentTag, DocumentVersionId,
};
use common::v1::types::pagination::{PaginationQuery, PaginationResponse};
use common::v1::types::util::Time;
use common::v1::types::{ChannelId, DocumentBranchId, DocumentTagId, UserId};
use sqlx::{query, query_as, query_scalar, Acquire};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct DbDocumentBranch {
    id: DocumentBranchId,
    document_id: ChannelId,
    creator_id: UserId,
    name: Option<String>,
    created_at: time::PrimitiveDateTime,
    is_default: bool,
    private: bool,
    state: DocumentBranchState,
    parent_branch_id: Option<Uuid>,
    parent_seq: Option<i64>,
}

impl From<DbDocumentBranch> for DocumentBranch {
    fn from(row: DbDocumentBranch) -> Self {
        Self {
            id: row.id,
            document_id: row.document_id,
            creator_id: row.creator_id,
            name: row.name,
            created_at: Time::from(row.created_at),
            default: row.is_default,
            private: row.private,
            state: row.state,
            parent_id: row.parent_branch_id.map(|branch_id| DocumentVersionId {
                branch_id: branch_id.into(),
                seq: row.parent_seq.unwrap_or(0) as u64,
            }),
        }
    }
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "branch_state")]
pub enum DbBranchState {
    Active,
    Closed,
    Merged,
}

impl From<DocumentBranchState> for DbBranchState {
    fn from(value: DocumentBranchState) -> Self {
        match value {
            DocumentBranchState::Active => DbBranchState::Active,
            DocumentBranchState::Closed => DbBranchState::Closed,
            DocumentBranchState::Merged => DbBranchState::Merged,
        }
    }
}

impl From<DbBranchState> for DocumentBranchState {
    fn from(value: DbBranchState) -> Self {
        match value {
            DbBranchState::Active => DocumentBranchState::Active,
            DbBranchState::Closed => DocumentBranchState::Closed,
            DbBranchState::Merged => DocumentBranchState::Merged,
        }
    }
}

#[derive(sqlx::FromRow)]
struct DbDocumentTag {
    id: DocumentTagId,
    branch_id: DocumentBranchId,
    revision_seq: i64,
    creator_id: Option<Uuid>,
    created_at: time::PrimitiveDateTime,
    updated_at: time::PrimitiveDateTime,
    summary: String,
    description: Option<String>,
}

impl From<DbDocumentTag> for DocumentTag {
    fn from(row: DbDocumentTag) -> Self {
        Self {
            id: row.id,
            branch_id: row.branch_id,
            revision_seq: row.revision_seq as u64,
            creator_id: row.creator_id.map(|i| i.into()),
            created_at: Time::from(row.created_at),
            updated_at: Time::from(row.updated_at),
            summary: row.summary,
            description: row.description,
        }
    }
}

#[async_trait]
impl DataDocument for Postgres {
    async fn document_compact(
        &self,
        context_id: EditContextId,
        last_snapshot_id: Uuid,
        last_seq: u32,
        snapshot: Vec<u8>,
    ) -> Result<()> {
        let (document_id, branch_id) = context_id;
        query!(
            r#"
            INSERT INTO document_snapshot (id, document_id, branch_id, snapshot, seq)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            last_snapshot_id,
            document_id.into_inner(),
            branch_id.into_inner(),
            snapshot,
            last_seq as i32
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn document_load(&self, context_id: EditContextId) -> Result<DehydratedDocument> {
        let (_, branch_id) = context_id;
        let snapshot = query!(
            r#"
            SELECT id, snapshot, seq
            FROM document_snapshot
            WHERE branch_id = $1
            ORDER BY seq DESC
            LIMIT 1
            "#,
            branch_id.into_inner()
        )
        .fetch_optional(&self.pool)
        .await?;

        let (last_snapshot, start_seq) = match snapshot {
            Some(row) => (row.snapshot, row.seq),
            None => return Err(Error::NotFound),
        };

        let updates = query!(
            r#"
            SELECT data
            FROM document_update
            WHERE branch_id = $1 AND seq > $2
            ORDER BY seq ASC
            "#,
            branch_id.into_inner(),
            start_seq
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(DehydratedDocument {
            last_snapshot,
            snapshot_seq: start_seq as u32,
            changes: updates.into_iter().map(|row| row.data).collect(),
        })
    }

    async fn document_create(
        &self,
        context_id: EditContextId,
        creator_id: UserId,
        snapshot: Vec<u8>,
    ) -> Result<()> {
        let (document_id, branch_id) = context_id;

        let mut tx = self.pool.begin().await?;

        query!(
            r#"
            INSERT INTO document_branch (id, document_id, creator_id, is_default, state)
            VALUES ($1, $2, $3, true, 'Active'::branch_state)
            ON CONFLICT (id) DO NOTHING
            "#,
            branch_id.into_inner(),
            document_id.into_inner(),
            creator_id.into_inner(),
        )
        .execute(&mut *tx)
        .await?;

        // Create initial snapshot
        let snapshot_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        query!(
            r#"
            INSERT INTO document_snapshot (id, document_id, branch_id, snapshot, seq)
            VALUES ($1, $2, $3, $4, 0)
            "#,
            snapshot_id,
            document_id.into_inner(),
            branch_id.into_inner(),
            snapshot
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    async fn document_update(
        &self,
        context_id: EditContextId,
        author_id: UserId,
        update: Vec<u8>,
    ) -> Result<u32> {
        let (document_id, branch_id) = context_id;
        let mut tx = self.pool.begin().await?;

        // get latest snapshot
        let snapshot = query!(
            r#"
            SELECT id, seq
            FROM document_snapshot
            WHERE branch_id = $1
            ORDER BY seq DESC
            LIMIT 1
            "#,
            branch_id.into_inner()
        )
        .fetch_optional(&mut *tx)
        .await?;

        let (snapshot_id, snapshot_seq) = match snapshot {
            Some(row) => (row.id, row.seq),
            None => return Err(Error::NotFound),
        };

        // get max update seq
        let max_update_seq = query!(
            r#"
            SELECT max(seq) as seq
            FROM document_update
            WHERE branch_id = $1
            "#,
            branch_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?
        .seq;

        let new_seq = max_update_seq.unwrap_or(snapshot_seq) + 1;

        query!(
            r#"
            INSERT INTO document_update (document_id, branch_id, snapshot_id, seq, data, author_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            document_id.into_inner(),
            branch_id.into_inner(),
            snapshot_id,
            new_seq,
            update,
            author_id.into_inner()
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(new_seq as u32)
    }

    async fn document_fork(
        &self,
        context_id: EditContextId,
        creator_id: UserId,
        create: DocumentBranchCreate,
    ) -> Result<DocumentBranchId> {
        let (document_id, parent_branch_id) = context_id;

        let mut tx = self.pool.begin().await?;

        let count: i64 = query_scalar!(
            "SELECT count(*) FROM document_branch WHERE document_id = $1 AND state = 'Active'::branch_state",
            document_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0);

        if count as usize >= crate::consts::MAX_DOCUMENT_BRANCHES {
            return Err(Error::BadRequest(format!(
                "too many active branches (max {})",
                crate::consts::MAX_DOCUMENT_BRANCHES
            )));
        }

        let parent_seq: i64 = query_scalar!(
            r#"
            SELECT COALESCE(
                (SELECT MAX(seq) FROM document_update WHERE branch_id = $1),
                (SELECT MAX(seq) FROM document_snapshot WHERE branch_id = $1),
                0
            )
            "#,
            parent_branch_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0) as i64;

        let branch_id = DocumentBranchId::new();

        query!(
            r#"
            INSERT INTO document_branch (id, document_id, creator_id, name, private, parent_branch_id, parent_seq, state)
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'Active'::branch_state)
            "#,
            branch_id.into_inner(),
            document_id.into_inner(),
            creator_id.into_inner(),
            create.name,
            create.private,
            parent_branch_id.into_inner(),
            parent_seq
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(branch_id)
    }

    async fn document_branch_get(
        &self,
        _document_id: ChannelId,
        branch_id: DocumentBranchId,
    ) -> Result<DocumentBranch> {
        let branch = query_as!(
            DbDocumentBranch,
            r#"
            SELECT id, document_id, creator_id, name, created_at, is_default, private, state as "state: DbBranchState", parent_branch_id, parent_seq
            FROM document_branch
            WHERE id = $1
            "#,
            branch_id.into_inner()
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::NotFound)?;

        Ok(branch.into())
    }

    async fn document_branch_update(
        &self,
        _document_id: ChannelId,
        branch_id: DocumentBranchId,
        patch: DocumentBranchPatch,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if let Some(name) = patch.name {
            query!(
                "UPDATE document_branch SET name = $1 WHERE id = $2",
                name,
                branch_id.into_inner()
            )
            .execute(&mut *tx)
            .await?;
        }

        if patch.private {
            query!(
                "UPDATE document_branch SET private = true WHERE id = $1",
                branch_id.into_inner()
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn document_branch_set_state(
        &self,
        _document_id: ChannelId,
        branch_id: DocumentBranchId,
        status: DocumentBranchState,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if status == DocumentBranchState::Active {
            let document_id = query_scalar!(
                "SELECT document_id FROM document_branch WHERE id = $1",
                branch_id.into_inner()
            )
            .fetch_one(&mut *tx)
            .await?;

            let count: i64 = query_scalar!(
                "SELECT count(*) FROM document_branch WHERE document_id = $1 AND state = 'Active'::branch_state",
                document_id
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(0);

            if count as usize >= crate::consts::MAX_DOCUMENT_BRANCHES {
                return Err(Error::BadRequest(format!(
                    "too many active branches (max {})",
                    crate::consts::MAX_DOCUMENT_BRANCHES
                )));
            }
        }

        let status: DbBranchState = status.into();
        query!(
            r#"UPDATE document_branch SET state = $1::branch_state WHERE id = $2"#,
            status as DbBranchState,
            branch_id.into_inner()
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn document_branch_list(&self, document_id: ChannelId) -> Result<Vec<DocumentBranch>> {
        let branches = query_as!(
            DbDocumentBranch,
            r#"
            SELECT id, document_id, creator_id, name, created_at, is_default, private, state as "state: DbBranchState", parent_branch_id, parent_seq
            FROM document_branch
            WHERE document_id = $1 AND state = 'Active'::branch_state
            ORDER BY created_at DESC
            "#,
            document_id.into_inner()
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(branches.into_iter().map(Into::into).collect())
    }

    async fn document_branch_paginate(
        &self,
        document_id: ChannelId,
        user_id: UserId,
        filter: DocumentBranchListParams,
        pagination: PaginationQuery<DocumentBranchId>,
    ) -> Result<PaginationResponse<DocumentBranch>> {
        let p: Pagination<_> = pagination.try_into()?;
        let states: Vec<DbBranchState> = filter.state.into_iter().map(Into::into).collect();

        // if states is empty, default to Active
        let states = if states.is_empty() {
            vec![DbBranchState::Active]
        } else {
            states
        };

        let branches = query_as!(
            DbDocumentBranch,
            r#"
            SELECT id, document_id, creator_id, name, created_at, is_default, private, state as "state: DbBranchState", parent_branch_id, parent_seq
            FROM document_branch
            WHERE document_id = $1 AND state = ANY($2::branch_state[])
            AND (private = false OR creator_id = $5)
            AND ($3::uuid IS NULL OR created_at < (SELECT created_at FROM document_branch WHERE id = $3))
            ORDER BY created_at DESC
            LIMIT $4
            "#,
            document_id.into_inner(),
            &states as &[DbBranchState],
            p.after.into_inner(),
            (p.limit + 1) as i64,
            user_id.into_inner()
        )
        .fetch_all(&self.pool)
        .await?;

        let total = query_scalar!(
            r#"
            SELECT count(*)
            FROM document_branch
            WHERE document_id = $1 AND state = ANY($2::branch_state[])
            AND (private = false OR creator_id = $3)
            "#,
            document_id.into_inner(),
            &states as &[DbBranchState],
            user_id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        let has_more = branches.len() > p.limit as usize;
        let mut items: Vec<DocumentBranch> = branches
            .into_iter()
            .take(p.limit as usize)
            .map(Into::into)
            .collect();

        if p.dir == PaginationDirection::B {
            items.reverse();
        }

        let cursor = items.last().map(|i| i.id.to_string());

        Ok(PaginationResponse {
            items,
            total: total as u64,
            has_more,
            cursor,
        })
    }

    async fn document_tag_create(
        &self,
        branch_id: DocumentBranchId,
        creator_id: UserId,
        summary: String,
        description: Option<String>,
        revision_seq: u64,
    ) -> Result<DocumentTagId> {
        let tag_id = query!(
            r#"
            INSERT INTO document_tag (branch_id, revision_seq, creator_id, summary, description)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            branch_id.into_inner(),
            revision_seq as i64,
            creator_id.into_inner(),
            summary,
            description
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(tag_id.id.into())
    }

    async fn document_tag_get(&self, tag_id: DocumentTagId) -> Result<DocumentTag> {
        let tag = query_as!(
            DbDocumentTag,
            r#"
            SELECT id, branch_id, revision_seq, creator_id, created_at, updated_at, summary, description
            FROM document_tag
            WHERE id = $1
            "#,
            tag_id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(tag.into())
    }

    async fn document_tag_update(
        &self,
        tag_id: DocumentTagId,
        summary: Option<String>,
        description: Option<Option<String>>,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        let tag = query!(
            r#"
            SELECT summary, description
            FROM document_tag
            WHERE id = $1
            FOR UPDATE
            "#,
            tag_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?;

        let new_summary = summary.unwrap_or(tag.summary);
        let new_description = description.unwrap_or(tag.description);

        query!(
            r#"
            UPDATE document_tag
            SET summary = $2, description = $3, updated_at = NOW()
            WHERE id = $1
            "#,
            tag_id.into_inner(),
            new_summary,
            new_description
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn document_tag_delete(&self, tag_id: DocumentTagId) -> Result<()> {
        query!(
            r#"
            DELETE FROM document_tag
            WHERE id = $1
            "#,
            tag_id.into_inner()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn document_tag_list(&self, branch_id: DocumentBranchId) -> Result<Vec<DocumentTag>> {
        let tags = query_as!(
            DbDocumentTag,
            r#"
            SELECT id, branch_id, revision_seq, creator_id, created_at, updated_at, summary, description
            FROM document_tag
            WHERE branch_id = $1
            ORDER BY created_at DESC
            "#,
            branch_id.into_inner()
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(tags.into_iter().map(Into::into).collect())
    }

    async fn document_tag_list_by_document(
        &self,
        document_id: ChannelId,
        user_id: UserId,
    ) -> Result<Vec<DocumentTag>> {
        let tags = query_as!(
            DbDocumentTag,
            r#"
            SELECT dt.id, dt.branch_id, dt.revision_seq, dt.creator_id, dt.created_at, dt.updated_at, dt.summary, dt.description
            FROM document_tag dt
            JOIN document_branch db ON dt.branch_id = db.id
            WHERE db.document_id = $1
            AND (db.private = false OR db.creator_id = $2)
            ORDER BY dt.created_at DESC
            "#,
            document_id.into_inner(),
            user_id.into_inner()
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(tags.into_iter().map(Into::into).collect())
    }
}
