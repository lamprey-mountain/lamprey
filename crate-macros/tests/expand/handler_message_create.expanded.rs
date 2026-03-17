use lamprey_macros::endpoint;
use common::v1::types::{UserWithRelationship, UserIdReq};
#[handler(message_create)]
async fn message_create(
    State(s): State<Arc<ServerState>>,
    auth: Auth,
    req: message_create::Request,
) -> Result<impl IntoResponse> {
    ::core::panicking::panic("not yet implemented")
}
