use async_trait::async_trait;

use crate::error::Result;
use crate::types::{Invite, InviteCode, RoomId, UserId};

use crate::data::DataInvite;

use super::Postgres;

#[async_trait]
impl DataInvite for Postgres {
    async fn invite_insert_room(
        &self,
        room_id: RoomId,
        creator_id: UserId,
        code: InviteCode,
    ) -> Result<Invite> {
        todo!()
    }
    async fn invite_select(&self, code: InviteCode) -> Result<Invite> {
        todo!()
    }
    async fn invite_delete(&self, code: InviteCode) -> Result<()> {
        todo!()
    }
}
