use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire};
use types::{
    InviteTarget, InviteWithMetadata, PaginationDirection, PaginationQuery, PaginationResponse,
    ThreadId,
};
use uuid::Uuid;

use crate::data::{DataInvite, DataRoom, DataThread, DataUser};
use crate::error::Result;
use crate::types::{DbInvite, Invite, InviteCode, RoomId, UserId};

use super::{Pagination, Postgres};

#[async_trait]
impl DataInvite for Postgres {
    async fn invite_insert_room(
        &self,
        room_id: RoomId,
        creator_id: UserId,
        code: InviteCode,
    ) -> Result<()> {
        query!(
            r#"
            insert into invite (target_type, target_id, code, creator_id)
            values ('room', $1, $2, $3)
        "#,
            room_id.into_inner(),
            code.0,
            creator_id.into_inner()
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
            select target_type, target_id, code, creator_id, created_at, expires_at, uses, max_uses
            from invite
            where code = $1
        "#,
            code.0
        )
        .fetch_one(&mut *tx)
        .await?;
        let target = match row.target_type.as_str() {
            "room" => {
                let room = self.room_get(RoomId::from(row.target_id)).await?;
                InviteTarget::Room { room }
            }
            "thread" => {
                let thread = self.thread_get(ThreadId::from(row.target_id), None).await?;
                let room = self.room_get(thread.room_id).await?;
                InviteTarget::Thread { room, thread }
            }
            "user" => {
                let user = self.user_get(UserId::from(row.target_id)).await?;
                InviteTarget::User { user }
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
            // TODO(#260): description
            None,
            false,
        );
        let invite_with_meta = InviteWithMetadata {
            invite,
            uses: row.uses.try_into().expect("invalid data in db"),
            max_uses: row
                .max_uses
                .map(|n| n.try_into().expect("invalid data in db")),
        };
        Ok(invite_with_meta)
    }

    async fn invite_delete(&self, code: InviteCode) -> Result<()> {
        query!("DELETE FROM invite WHERE code = $1", code.0)
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
            select target_type, target_id, code, creator_id, created_at, expires_at, uses, max_uses
            from invite
        	WHERE target_id = $1 AND code > $2 AND code < $3
        	ORDER BY (CASE WHEN $4 = 'f' THEN code END), code DESC LIMIT $5
        ",
            room_id.into_inner(),
            p.after.to_string(),
            p.before.to_string(),
            p.dir.to_string(),
            (p.limit + 1) as i32
        )
        .fetch_all(&mut *tx)
        .await?;
        let total = query_scalar!(
            "SELECT count(*) FROM invite WHERE target_id = $1",
            room_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.rollback().await?;
        let has_more = raw.len() > p.limit as usize;
        let mut items = vec![];
        let room = self.room_get(room_id).await?;
        for row in raw.into_iter().take(p.limit as usize) {
            assert_eq!(row.target_type, "room");
            assert_eq!(row.target_id, room_id.into_inner());
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
                // TODO(#260): description
                None,
                false,
            );
            let invite_with_meta = InviteWithMetadata {
                invite,
                uses: row.uses.try_into().expect("invalid data in db"),
                max_uses: row
                    .max_uses
                    .map(|n| n.try_into().expect("invalid data in db")),
            };
            items.push(invite_with_meta);
        }
        if p.dir == PaginationDirection::B {
            items.reverse();
        }
        Ok(PaginationResponse {
            items,
            total: total.unwrap_or(0) as u64,
            has_more,
        })
    }

    async fn invite_incr_use(&self, target_id: Uuid) -> Result<()> {
        query!(
            "update invite set uses = uses + 1 where target_id = $1",
            target_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
