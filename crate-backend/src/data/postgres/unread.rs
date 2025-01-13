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
impl DataUnread for Postgres {
    async fn unread_mark_thread(
&self,        user_id: UserId,
        thread_id: ThreadId,
    ) -> Result<MessageVerId> {
        todo!()
    }

    async fn unread_mark_message(
     &self,   user_id: UserId,
        thread_id: ThreadId,
        version_id: MessageVerId,
    ) -> Result<MessageVerId> {
        todo!()
    }
}
