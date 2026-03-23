use lamprey_macros::endpoint;

/// Permission overwrite
#[endpoint(
    put,
    path = "/channel/{channel_id}/permission/{overwrite_id}",
    tags = ["channel"],
    scopes = [Full],
    permissions = [RoleManage],
    audit_log_events = ["PermissionOverwriteCreate", "PermissionOverwriteUpdate"],
    response(NO_CONTENT, description = "success"),
)]
pub mod permission_set {
    use crate::v1::types::{ChannelId, PermissionOverwriteSet};
    use uuid::Uuid;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub overwrite_id: Uuid,

        #[json]
        pub overwrite: PermissionOverwriteSet,
    }

    pub struct Response {}
}

/// Permission delete
#[endpoint(
    delete,
    path = "/channel/{channel_id}/permission/{overwrite_id}",
    tags = ["channel"],
    scopes = [Full],
    permissions = [RoleManage],
    audit_log_events = ["PermissionOverwriteDelete"],
    response(NO_CONTENT, description = "success"),
)]
pub mod permission_remove {
    use crate::v1::types::ChannelId;
    use uuid::Uuid;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub overwrite_id: Uuid,
    }

    pub struct Response {}
}
