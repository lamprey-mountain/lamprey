use async_trait::async_trait;
use sqlx::{query_as, Acquire};
use types::{InviteTarget, InviteWithMetadata, ThreadId};

use crate::error::Result;
use crate::types::{DbInvite, Invite, InviteCode, RoomId, UserId};

use crate::data::{DataInvite, DataRoom, DataThread, DataUser};

use super::Postgres;

#[async_trait]
impl DataInvite for Postgres {
    async fn invite_insert_room(
        &self,
        _room_id: RoomId,
        _creator_id: UserId,
        _code: InviteCode,
    ) -> Result<InviteWithMetadata> {
        todo!()
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
                let room = self.room_get(RoomId(row.target_id)).await?;
                InviteTarget::Room { room }
            },
            "thread" => {
                let thread = self.thread_get(ThreadId(row.target_id), None).await?;
                let room = self.room_get(thread.room_id).await?;
                InviteTarget::Thread {
                    room,
                    thread,
                }
            },
            "user" => {
                let user = self.user_get(UserId(row.target_id)).await?;
                InviteTarget::User { user }
            },
            _ => panic!("invalid data in db"),
        };
        let creator = self.user_get(UserId(row.creator_id)).await?;
        let invite = Invite {
            code,
            target,
            creator,
            created_at: row.created_at.assume_utc(),
            expires_at: row.expires_at.map(|t| t.assume_utc()),
        };
        let invite_with_meta = InviteWithMetadata {
          invite,
          uses: row.uses.try_into().expect("invalid data in db"),
          max_uses: row.max_uses.map(|n| n.try_into().expect("invalid data in db")),
        };
        Ok(invite_with_meta)
    }

    async fn invite_delete(&self, _code: InviteCode) -> Result<()> {
        todo!()
    }
}
