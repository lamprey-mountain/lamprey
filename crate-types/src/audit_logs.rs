use crate::{AuditLogId, MessageSync, RoomId, UserId};

use serde::{Deserialize, Serialize};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuditLog {
    /// Unique id idenfitying this entry
    pub id: AuditLogId,

    /// Room this happened in
    pub room_id: RoomId,

    /// User who caused this entry to be created
    pub user_id: UserId,

    /// User supplied reason why this happened
    pub reason: Option<String>,

    /// Generated sync payload (sent in websocket)
    pub payload: MessageSync,

    /// The previous payload, or None if this resource is newly created
    // theres probably a better way to do this, but its the best solution i could think of for now
    pub payload_prev: Option<MessageSync>,
}
