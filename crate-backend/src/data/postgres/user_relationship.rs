use async_trait::async_trait;
use common::v1::types::{
    Ignore, PaginationDirection, PaginationQuery, PaginationResponse, Relationship,
    RelationshipPatch, RelationshipType, RelationshipWithUserId,
};
use sqlx::{query, query_as, query_scalar, Acquire};
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::data::postgres::Pagination;
use crate::error::Result;
use crate::gen_paginate;
use crate::types::UserId;

use crate::data::DataUserRelationship;

use super::Postgres;

#[derive(sqlx::Type)]
#[sqlx(type_name = "user_relationship_type")]
enum DbUserRelType {
    Friend,
    Outgoing,
    Incoming,
    Block,
}

struct DbUserRel {
    rel: Option<DbUserRelType>,
    ignore: bool,
    ignore_until: Option<PrimitiveDateTime>,
}

struct DbUserRelWithId {
    rel: Option<DbUserRelType>,
    ignore: bool,
    ignore_until: Option<PrimitiveDateTime>,
    user_id: Uuid,
}

impl From<DbUserRelType> for RelationshipType {
    fn from(value: DbUserRelType) -> Self {
        match value {
            DbUserRelType::Friend => RelationshipType::Friend,
            DbUserRelType::Outgoing => RelationshipType::Outgoing,
            DbUserRelType::Incoming => RelationshipType::Incoming,
            DbUserRelType::Block => RelationshipType::Block,
        }
    }
}

impl From<RelationshipType> for DbUserRelType {
    fn from(value: RelationshipType) -> Self {
        match value {
            RelationshipType::Friend => DbUserRelType::Friend,
            RelationshipType::Outgoing => DbUserRelType::Outgoing,
            RelationshipType::Incoming => DbUserRelType::Incoming,
            RelationshipType::Block => DbUserRelType::Block,
        }
    }
}

impl From<Relationship> for DbUserRel {
    fn from(value: Relationship) -> Self {
        DbUserRel {
            rel: value.relation.map(Into::into),
            ignore: value.ignore.is_some(),
            ignore_until: value.ignore.and_then(|i| i.until.map(Into::into)),
        }
    }
}

impl From<DbUserRel> for Relationship {
    fn from(value: DbUserRel) -> Self {
        Relationship {
            relation: value.rel.map(Into::into),
            ignore: if value.ignore {
                Some(Ignore {
                    until: value.ignore_until.map(|t| t.into()),
                })
            } else {
                None
            },
        }
    }
}

impl From<DbUserRelWithId> for RelationshipWithUserId {
    fn from(value: DbUserRelWithId) -> Self {
        RelationshipWithUserId {
            user_id: value.user_id.into(),
            inner: Relationship {
                relation: value.rel.map(Into::into),
                ignore: if value.ignore {
                    Some(Ignore {
                        until: value.ignore_until.map(|t| t.into()),
                    })
                } else {
                    None
                },
            },
        }
    }
}

#[async_trait]
impl DataUserRelationship for Postgres {
    async fn user_relationship_put(
        &self,
        user_id: UserId,
        other_id: UserId,
        rel: Relationship,
    ) -> Result<()> {
        let rel: DbUserRel = rel.into();
        query!(
            r#"
            INSERT INTO user_relationship (user_id, other_id, rel, ignore, ignore_until)
            VALUES ($1, $2, $3, $4, $5)
			ON CONFLICT ON CONSTRAINT user_relationship_pkey DO UPDATE SET
    			rel = excluded.rel,
    			ignore = excluded.ignore,
    			ignore_until = excluded.ignore_until;
            "#,
            user_id.into_inner(),
            other_id.into_inner(),
            rel.rel as _,
            rel.ignore,
            rel.ignore_until,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn user_relationship_edit(
        &self,
        user_id: UserId,
        other_id: UserId,
        patch: RelationshipPatch,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let row = query_as!(
            DbUserRel,
            r#"
            SELECT rel as "rel: _", ignore, ignore_until FROM user_relationship
            WHERE user_id = $1 AND other_id = $2
            FOR UPDATE
            "#,
            user_id.into_inner(),
            other_id.into_inner(),
        )
        .fetch_optional(&mut *tx)
        .await?;
        let rel: Relationship = row.map(Into::into).unwrap_or_default();
        let rel = Relationship {
            relation: patch.relation.unwrap_or(rel.relation),
            ignore: patch.ignore.unwrap_or(rel.ignore),
        };
        let rel: DbUserRel = rel.into();
        query!(
            r#"
            INSERT INTO user_relationship (user_id, other_id, rel, ignore, ignore_until)
            VALUES ($1, $2, $3, $4, $5)
			ON CONFLICT ON CONSTRAINT user_relationship_pkey DO UPDATE SET
    			rel = excluded.rel,
    			ignore = excluded.ignore,
    			ignore_until = excluded.ignore_until;
            "#,
            user_id.into_inner(),
            other_id.into_inner(),
            rel.rel as _,
            rel.ignore,
            rel.ignore_until,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn user_relationship_delete(&self, user_id: UserId, other_id: UserId) -> Result<()> {
        query!(
            r#"DELETE FROM user_relationship WHERE user_id = $1 AND other_id = $2"#,
            user_id.into_inner(),
            other_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn user_relationship_get(
        &self,
        user_id: UserId,
        other_id: UserId,
    ) -> Result<Option<Relationship>> {
        let row = query_as!(
            DbUserRel,
            r#"
            SELECT rel as "rel: _", ignore, ignore_until FROM user_relationship
            WHERE user_id = $1 AND other_id = $2
            "#,
            user_id.into_inner(),
            other_id.into_inner(),
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    async fn user_relationship_list_blocked(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbUserRelWithId,
                r#"
                SELECT rel as "rel: _", ignore, ignore_until, other_id as user_id FROM user_relationship
            	WHERE user_id = $1 AND other_id > $2 AND other_id < $3 AND rel = 'Block'
            	ORDER BY (CASE WHEN $4 = 'f' THEN other_id END), other_id DESC LIMIT $5
                "#,
                user_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"SELECT count(*) FROM user_relationship WHERE user_id = $1 AND rel = 'Block'"#,
                user_id.into_inner(),
            ),
            |i: &RelationshipWithUserId| i.user_id.to_string()
        )
    }

    async fn user_relationship_list_friends(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbUserRelWithId,
                r#"
                SELECT rel as "rel: _", ignore, ignore_until, other_id as user_id FROM user_relationship
                WHERE user_id = $1 AND other_id > $2 AND other_id < $3 AND rel = 'Friend'
                ORDER BY (CASE WHEN $4 = 'f' THEN other_id END), other_id DESC LIMIT $5
                "#,
                user_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"SELECT count(*) FROM user_relationship WHERE user_id = $1 AND rel = 'Friend'"#,
                user_id.into_inner(),
            ),
            |i: &RelationshipWithUserId| i.user_id.to_string()
        )
    }

    async fn user_relationship_list_pending(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbUserRelWithId,
                r#"
                SELECT rel as "rel: _", ignore, ignore_until, other_id as user_id FROM user_relationship
                WHERE user_id = $1 AND other_id > $2 AND other_id < $3 AND (rel = 'Incoming' OR rel = 'Outgoing')
                ORDER BY (CASE WHEN $4 = 'f' THEN other_id END), other_id DESC LIMIT $5
                "#,
                user_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"SELECT count(*) FROM user_relationship WHERE user_id = $1 AND (rel = 'Incoming' OR rel = 'Outgoing')"#,
                user_id.into_inner(),
            ),
            |i: &RelationshipWithUserId| i.user_id.to_string()
        )
    }

    async fn user_relationship_list_ignored(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RelationshipWithUserId>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbUserRelWithId,
                r#"
                SELECT rel as "rel: _", ignore, ignore_until, other_id as user_id FROM user_relationship
                WHERE user_id = $1 AND other_id > $2 AND other_id < $3 AND ignore = TRUE
                ORDER BY (CASE WHEN $4 = 'f' THEN other_id END), other_id DESC LIMIT $5
                "#,
                user_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"SELECT count(*) FROM user_relationship WHERE user_id = $1 AND ignore = TRUE"#,
                user_id.into_inner(),
            ),
            |i: &RelationshipWithUserId| i.user_id.to_string()
        )
    }
}
