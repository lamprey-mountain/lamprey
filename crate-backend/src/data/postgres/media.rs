use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire, PgPool};
use tracing::info;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    Identifier, Media, MediaId, MediaLink, MediaLinkType, Message, MessageCreate, MessageId, MessageType, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse, Permission, Role, RoleCreate, RoleId, Room, RoomCreate, RoomId, RoomMemberPut, RoomPatch, RoomVerId, Thread, ThreadCreate, ThreadId, UserId
};

use crate::data::{
    DataMedia, DataMessage, DataPermission, DataRole, DataRoleMember, DataRoom, DataRoomMember, DataThread, DataUnread
};

use super::{Pagination, Postgres};

#[async_trait]
impl DataMedia for Postgres {
    async fn media_insert(&self, user_id: UserId, media: Media) -> Result<Media> {
        todo!()
    }

    async fn media_select(&self, media_id: MediaId) -> Result<Media> {
        todo!()
    }

    async fn media_link_insert(
        &self,
        media_id: MediaId,
        thing_id: Uuid,
        link_type: MediaLinkType,
    ) -> Result<()> {
        todo!()
    }

    async fn media_link_select(&self, media_id: MediaId) -> Result<Vec<MediaLink>> {
        todo!()
    }

    async fn media_link_delete(
        &self,
        thing_id: Uuid,
        link_type: MediaLinkType,
    ) -> Result<()> {
        todo!()
    }

    async fn media_link_delete_all(&self, thing_id: Uuid) -> Result<()> {
        todo!()
    }
}
