use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::MessageVerId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadTypeForumTreePublic {
    pub last_version_id: MessageVerId,
    pub message_count: u64,
    pub root_message_count: u64,
    // maybe? pub user_limit: u64,
}
