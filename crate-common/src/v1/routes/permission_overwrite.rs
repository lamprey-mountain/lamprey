use lamprey_macros::endpoint;

/// Permission overwrite
#[endpoint(
    put,
    path = "/channel/{channel_id}/permission/{overwrite_id}",
    tags = ["channel"],
    scopes = [Full],
    permissions = [RoleManage],
    response(NO_CONTENT, description = "success"),
)]
pub mod permission_overwrite {
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

    pub struct Response;
}
