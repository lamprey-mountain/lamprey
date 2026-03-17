use lamprey_macros::endpoint;
use common::v1::types::{UserWithRelationship, UserIdReq};

/// Message create
///
/// Send a message to a channel
#[endpoint(
    post,
    path = "/channel/{channel_id}/message",
    tags = ["message"],
    scopes = ["full"],
    permissions = ["MessageCreate"],
    permissions_optional = ["MessageAttachments", "MessageEmbeds", "MemberBridge"],
    response(status = CREATED, body = Message, description = "success"),
    response(status = OK, body = Message, description = "already created with same nonce"),
    errors(UnknownChannel),
)]
pub mod message_create {
    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub body: MessageCreate,
    }

    pub struct Response {
        #[json]
        pub message: Message,
    }
}

#[handler(message_create)]
async fn message_create(
    State(s): State<Arc<ServerState>>,
    auth: Auth,
    req: message_create::Request,
) -> Result<impl IntoResponse> {
    todo!()
}
