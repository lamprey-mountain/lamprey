use async_trait::async_trait;
use common::v1::types::util::Time;
use common::v1::types::{
    InviteTarget, InviteWithMetadata, PaginationDirection, PaginationQuery, PaginationResponse,
    ThreadId,
};
use sqlx::{query, query_as, query_scalar, Acquire};

use crate::data::{DataInvite, DataRoom, DataThread, DataUser};
use crate::error::{Error, Result};
use crate::types::{DbInvite, Invite, InviteCode, RoomId, UserId};
use common::v1::types::InvitePatch;
use time::PrimitiveDateTime;

use super::{Pagination, Postgres};

#[async_trait]
impl DataInvite for Postgres {
    async fn invite_insert_room(
        &self,
        room_id: RoomId,
        creator_id: UserId,
        code: InviteCode,
        expires_at: Option<Time>,
        max_uses: Option<u16>,
    ) -> Result<()> {
        query!(
            r#"
            insert into invite (target_type, target_id, code, creator_id, expires_at, max_uses)
            values ('room', $1, $2, $3, $4, $5)
        "#,
            *room_id,
            code.0,
            *creator_id,
            expires_at.map(|t| PrimitiveDateTime::new(t.date(), t.time())),
            max_uses.map(|n| n as i32),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn invite_insert_server(
        &self,
        creator_id: UserId,
        code: InviteCode,
        expires_at: Option<Time>,
        max_uses: Option<u16>,
    ) -> Result<()> {
        query!(
            r#"
            insert into invite (target_type, code, creator_id, expires_at, max_uses)
            values ('server', $1, $2, $3, $4)
        "#,
            code.0,
            creator_id.into_inner(),
            expires_at.map(|t| PrimitiveDateTime::new(t.date(), t.time())),
            max_uses.map(|n| n as i32),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn invite_select(&self, code: InviteCode) -> Result<InviteWithMetadata> {
        let mut conn = self.pool.begin().await?;
        let mut tx = conn.begin().await?;
        let row = query_as!(
            DbInvite,
            r#"
            select target_type, target_id, code, creator_id, created_at, expires_at, uses, max_uses, description
            from invite
            where code = $1 and deleted_at is null
        "#,
            code.0
        )
        .fetch_one(&mut *tx)
        .await?;
        let target = match row.target_type.as_str() {
            "room" => {
                let room = self.room_get(RoomId::from(row.target_id.unwrap())).await?;
                InviteTarget::Room { room }
            }
            "thread" => {
                // FIXME: get thread via services
                let thread = self
                    .thread_get(ThreadId::from(row.target_id.unwrap()))
                    .await?;
                let room_id = thread.room_id.ok_or_else(|| Error::NotFound)?;
                let room = self.room_get(room_id).await?;
                InviteTarget::Thread {
                    room,
                    thread: Box::new(thread),
                }
            }
            "server" => InviteTarget::Server,
            "user" => {
                let user = self.user_get(UserId::from(row.target_id.unwrap())).await?;
                InviteTarget::User {
                    user: Box::new(user),
                }
            }
            _ => panic!("invalid data in db"),
        };
        let creator = self.user_get(UserId::from(row.creator_id)).await?;
        let creator_id = creator.id;
        let invite = Invite::new(
            code,
            target,
            creator,
            creator_id,
            row.created_at.assume_utc().into(),
            row.expires_at.map(|t| t.assume_utc().into()),
            row.description,
            false,
        );
        let invite_with_meta = InviteWithMetadata {
            invite,
            uses: row.uses.try_into().expect("invalid data in db"),
            max_uses: row
                .max_uses
                .map(|n| n.try_into().expect("invalid data in db"))
                as Option<u16>,
        };
        Ok(invite_with_meta)
    }

    async fn invite_delete(&self, code: InviteCode) -> Result<()> {
        query!(
            "update invite set deleted_at = now() where code = $1",
            code.0
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn invite_list_room(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>> {
        let p: Pagination<_> = paginate.try_into()?;
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let raw = query_as!(
            DbInvite,
            "
            select target_type, target_id, code, creator_id, created_at, expires_at, uses, max_uses, description
            from invite
        	WHERE target_id = $1 AND code > $2 AND code < $3 and deleted_at is null
        	ORDER BY (CASE WHEN $4 = 'f' THEN code END), code DESC LIMIT $5
        ",
            *room_id,
            p.after.to_string(),
            p.before.to_string(),
            p.dir.to_string(),
            (p.limit + 1) as i32
        )
        .fetch_all(&mut *tx)
        .await?;
        let total = query_scalar!(
            "SELECT count(*) FROM invite WHERE target_id = $1 and deleted_at is null",
            *room_id
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.rollback().await?;
        let has_more = raw.len() > p.limit as usize;
        let mut items = vec![];
        let room = self.room_get(room_id).await?;
        for row in raw.into_iter().take(p.limit as usize) {
            assert_eq!(row.target_type, "room");
            assert_eq!(row.target_id, Some(*room_id));
            let target = InviteTarget::Room { room: room.clone() };
            let creator = self.user_get(UserId::from(row.creator_id)).await?;
            let creator_id = creator.id;
            let invite = Invite::new(
                InviteCode(row.code),
                target,
                creator,
                creator_id,
                row.created_at.assume_utc().into(),
                row.expires_at.map(|t| t.assume_utc().into()),
                row.description,
                false,
            );
            let invite_with_meta = InviteWithMetadata {
                invite,
                uses: row.uses.try_into().expect("invalid data in db"),
                max_uses: row
                    .max_uses
                    .map(|n| n.try_into().expect("invalid data in db"))
                    as Option<u16>,
            };
            items.push(invite_with_meta);
        }
        if p.dir == PaginationDirection::B {
            items.reverse();
        }
        let cursor = items.last().map(|i| i.invite.code.to_string());
        Ok(PaginationResponse {
            items,
            total: total.unwrap_or(0) as u64,
            has_more,
            cursor,
        })
    }

    async fn invite_list_server(
        &self,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>> {
        let p: Pagination<_> = paginate.try_into()?;
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let raw = query_as!(
            DbInvite,
            r#"
            select target_type, target_id, code, creator_id, created_at, expires_at, uses, max_uses, description
            from invite
        	WHERE target_type = 'server' AND code > $1 AND code < $2 and deleted_at is null
        	ORDER BY (CASE WHEN $3 = 'f' THEN code END), code DESC LIMIT $4
        "#,
            p.after.to_string(),
            p.before.to_string(),
            p.dir.to_string(),
            (p.limit + 1) as i32
        )
        .fetch_all(&mut *tx)
        .await?;
        let total = query_scalar!(
            "SELECT count(*) FROM invite WHERE target_type = 'server' and deleted_at is null",
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.rollback().await?;
        let has_more = raw.len() > p.limit as usize;
        let mut items = vec![];
        for row in raw.into_iter().take(p.limit as usize) {
            assert_eq!(row.target_type, "server");
            let target = InviteTarget::Server;
            let creator = self.user_get(UserId::from(row.creator_id)).await?;
            let creator_id = creator.id;
            let invite = Invite::new(
                InviteCode(row.code),
                target,
                creator,
                creator_id,
                row.created_at.assume_utc().into(),
                row.expires_at.map(|t| t.assume_utc().into()),
                row.description,
                false,
            );
            let invite_with_meta = InviteWithMetadata {
                invite,
                uses: row.uses.try_into().expect("invalid data in db"),
                max_uses: row
                    .max_uses
                    .map(|n| n.try_into().expect("invalid data in db"))
                    as Option<u16>,
            };
            items.push(invite_with_meta);
        }
        if p.dir == PaginationDirection::B {
            items.reverse();
        }
        let cursor = items.last().map(|i| i.invite.code.to_string());
        Ok(PaginationResponse {
            items,
            total: total.unwrap_or(0) as u64,
            has_more,
            cursor,
        })
    }

    async fn invite_list_server_by_creator(
        &self,
        creator_id: UserId,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>> {
        let p: Pagination<_> = paginate.try_into()?;
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let raw = query_as!(
            DbInvite,
            r#"
            select target_type, target_id, code, creator_id, created_at, expires_at, uses, max_uses, description
            from invite
        	WHERE target_type = 'server' AND creator_id = $1 AND code > $2 AND code < $3 and deleted_at is null
        	ORDER BY (CASE WHEN $4 = 'f' THEN code END), code DESC LIMIT $5
        "#,
            *creator_id,
            p.after.to_string(),
            p.before.to_string(),
            p.dir.to_string(),
            (p.limit + 1) as i32
        )
        .fetch_all(&mut *tx)
        .await?;
        let total = query_scalar!(
            "SELECT count(*) FROM invite WHERE target_type = 'server' and creator_id = $1 and deleted_at is null",
            *creator_id
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.rollback().await?;
        let has_more = raw.len() > p.limit as usize;
        let mut items = vec![];
        for row in raw.into_iter().take(p.limit as usize) {
            assert_eq!(row.target_type, "server");
            let target = InviteTarget::Server;
            let creator = self.user_get(UserId::from(row.creator_id)).await?;
            let creator_id = creator.id;
            let invite = Invite::new(
                InviteCode(row.code),
                target,
                creator,
                creator_id,
                row.created_at.assume_utc().into(),
                row.expires_at.map(|t| t.assume_utc().into()),
                row.description,
                false,
            );
            let invite_with_meta = InviteWithMetadata {
                invite,
                uses: row.uses.try_into().expect("invalid data in db"),
                max_uses: row
                    .max_uses
                    .map(|n| n.try_into().expect("invalid data in db"))
                    as Option<u16>,
            };
            items.push(invite_with_meta);
        }
        if p.dir == PaginationDirection::B {
            items.reverse();
        }
        let cursor = items.last().map(|i| i.invite.code.to_string());
        Ok(PaginationResponse {
            items,
            total: total.unwrap_or(0) as u64,
            has_more,
            cursor,
        })
    }

    async fn invite_insert_user(
        &self,
        user_id: UserId,
        creator_id: UserId,
        code: InviteCode,
        expires_at: Option<Time>,
        max_uses: Option<u16>,
    ) -> Result<()> {
        query!(
            r#"
            insert into invite (target_type, target_id, code, creator_id, expires_at, max_uses)
            values ('user', $1, $2, $3, $4, $5)
        "#,
            *user_id,
            code.0,
            *creator_id,
            expires_at.map(|t| PrimitiveDateTime::new(t.date(), t.time())),
            max_uses.map(|n| n as i32),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn invite_list_user(
        &self,
        user_id: UserId,
        paginate: PaginationQuery<InviteCode>,
    ) -> Result<PaginationResponse<InviteWithMetadata>> {
        let p: Pagination<_> = paginate.try_into()?;
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let raw = query_as!(
            DbInvite,
            r#"
            select target_type, target_id, code, creator_id, created_at, expires_at, uses, max_uses, description
            from invite
        	WHERE target_type = 'user' AND target_id = $1 AND code > $2 AND code < $3 and deleted_at is null
        	ORDER BY (CASE WHEN $4 = 'f' THEN code END), code DESC LIMIT $5
        "#,
            *user_id,
            p.after.to_string(),
            p.before.to_string(),
            p.dir.to_string(),
            (p.limit + 1) as i32
        )
        .fetch_all(&mut *tx)
        .await?;
        let total = query_scalar!(
            "SELECT count(*) FROM invite WHERE target_type = 'user' and target_id = $1 and deleted_at is null",
            *user_id
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.rollback().await?;
        let has_more = raw.len() > p.limit as usize;
        let mut items = vec![];
        for row in raw.into_iter().take(p.limit as usize) {
            assert_eq!(row.target_type, "user");
            let creator = self.user_get(UserId::from(row.target_id.unwrap())).await?;
            let target = InviteTarget::User {
                user: Box::new(creator.clone()),
            };
            let creator_id = creator.id;
            let invite = Invite::new(
                InviteCode(row.code),
                target,
                creator,
                creator_id,
                row.created_at.assume_utc().into(),
                row.expires_at.map(|t| t.assume_utc().into()),
                row.description,
                false,
            );
            let invite_with_meta = InviteWithMetadata {
                invite,
                uses: row.uses.try_into().expect("invalid data in db"),
                max_uses: row
                    .max_uses
                    .map(|n| n.try_into().expect("invalid data in db"))
                    as Option<u16>,
            };
            items.push(invite_with_meta);
        }
        if p.dir == PaginationDirection::B {
            items.reverse();
        }
        let cursor = items.last().map(|i| i.invite.code.to_string());
        Ok(PaginationResponse {
            items,
            total: total.unwrap_or(0) as u64,
            has_more,
            cursor,
        })
    }

    async fn invite_incr_use(&self, code: InviteCode) -> Result<()> {
        query!("update invite set uses = uses + 1 where code = $1", code.0)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn invite_update(
        &self,
        code: InviteCode,
        patch: InvitePatch,
    ) -> Result<InviteWithMetadata> {
        let mut conn = self.pool.begin().await?;
        let mut tx = conn.begin().await?;

        let invite = query_as!(
            DbInvite,
            r#"
            select target_type, target_id, code, creator_id, created_at, expires_at, uses, max_uses, description
            from invite
            where code = $1 and deleted_at is null
            FOR UPDATE
            "#,
            code.0
        )
        .fetch_one(&mut *tx)
        .await?;

        let expires_at = patch.expires_at.map_or(invite.expires_at, |ea| {
            ea.map(|t| {
                let inner = t.into_inner();
                PrimitiveDateTime::new(inner.date(), inner.time())
            })
        });
        let max_uses = patch
            .max_uses
            .map_or(invite.max_uses, |mu| mu.map(|u| u as i32));
        let description = patch.description.map_or(invite.description, |d| d);

        query!(
            r#"
            UPDATE invite
            SET expires_at = $1, max_uses = $2, description = $3
            WHERE code = $4
            "#,
            expires_at,
            max_uses,
            description,
            code.0
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.invite_select(code).await
    }
}
