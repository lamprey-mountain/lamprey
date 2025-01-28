use crate::{AuditLogId, MessageSync, RoomId, UserId};

use serde::{Deserialize, Serialize};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuditLog {
    pub id: AuditLogId,
    pub room_id: RoomId,
    pub user_id: UserId,
    pub reason: Option<String>,
    pub payload: MessageSync,
}
