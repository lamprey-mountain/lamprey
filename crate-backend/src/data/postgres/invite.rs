use async_trait::async_trait;

use crate::error::Result;
use crate::types::{Invite, InviteCode, RoomId, UserId};

use crate::data::DataInvite;

use super::Postgres;

#[async_trait]
impl DataInvite for Postgres {
    async fn invite_insert_room(
        &self,
        _room_id: RoomId,
        _creator_id: UserId,
        _code: InviteCode,
    ) -> Result<Invite> {
        todo!()
    }

    async fn invite_select(&self, _code: InviteCode) -> Result<Invite> {
        todo!()
    }

    async fn invite_delete(&self, _code: InviteCode) -> Result<()> {
        todo!()
    }
}
