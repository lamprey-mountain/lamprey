use lamprey_macros::endpoint;

/// Interaction create
#[endpoint(
    post,
    path = "/interaction",
    tags = ["interaction"],
    scopes = [Full],
    response(CREATED, body = Interaction, description = "Interaction created successfully"),
)]
pub mod interaction_create {
    use crate::v1::types::interactions::{Interaction, InteractionCreate};

    pub struct Request {
        #[header]
        pub idempotency_key: Option<String>,

        #[json]
        pub create: InteractionCreate,
    }

    pub struct Response {
        #[json]
        pub interaction: Interaction,
    }
}

/// Interaction respond
///
/// Respond to an interaction
#[endpoint(
    post,
    path = "/interaction/{interaction_id}/{token}/callback",
    tags = ["interaction"],
    response(OK, body = InteractionResponse, description = "Interaction response accepted"),
    response(ACCEPTED, description = "Interaction response accepted"),
)]
pub mod interaction_respond {
    use serde::{Deserialize, Serialize};
    use utoipa::{IntoParams, ToSchema};

    use crate::v1::types::InteractionId;
    use crate::v1::types::interactions::{InteractionResponse, InteractionResponseCreate};

    pub struct Request {
        #[path]
        pub interaction_id: InteractionId,

        #[path]
        pub token: String,

        #[query]
        pub query: InteractionResponseQueryParams,

        #[json]
        pub response: InteractionResponseCreate,
    }

    pub struct Response {
        // TODO: implement ?wait=true
        // #[json]
        // pub response: InteractionResponse,
    }

    #[derive(Debug, IntoParams, ToSchema, Serialize, Deserialize)]
    pub struct InteractionResponseQueryParams {
        /// whether to immediately return with 202 accepted or wait to return an `InteractionResponse`
        pub wait: bool,
    }
}

/// Interaction message create
///
/// Send another message (aka followup message)
#[endpoint(
    post,
    path = "/interaction/{interaction_id}/{token}/message",
    tags = ["interaction"],
    scopes = [Full],
    response(CREATED, body = Message, description = "Interaction message created successfully"),
)]
pub mod interaction_message_create {
    use crate::v1::types::{InteractionId, Message, MessageCreate};

    pub struct Request {
        #[path]
        pub interaction_id: InteractionId,

        #[path]
        pub token: String,

        #[json]
        pub message: MessageCreate,

        #[header]
        pub idempotency_key: Option<String>,
    }

    pub struct Response {
        #[json]
        pub message: Message,
    }
}

/// Interaction message get
///
/// Get a message from an interaction
#[endpoint(
    get,
    path = "/interaction/{interaction_id}/{token}/message/{message_id}",
    tags = ["interaction", "badge.public"],
    scopes = [Full],
    response(OK, body = Message, description = "Get interaction message success"),
)]
pub mod interaction_message_get {
    use crate::v1::types::{InteractionId, Message, misc::InteractionMessageReq};

    pub struct Request {
        #[path]
        pub interaction_id: InteractionId,

        #[path]
        pub token: String,

        #[path]
        pub message_id: InteractionMessageReq,
    }

    pub struct Response {
        #[json]
        pub message: Message,
    }
}

/// Interaction message edit
///
/// Edit a message from an interaction
#[endpoint(
    patch,
    path = "/interaction/{interaction_id}/{token}/message/{message_id}",
    tags = ["interaction"],
    scopes = [Full],
    response(OK, body = Message, description = "Edit interaction message success"),
    response(NO_CONTENT, description = "no change"),
)]
pub mod interaction_message_edit {
    use crate::v1::types::{InteractionId, Message, MessagePatch, misc::InteractionMessageReq};

    pub struct Request {
        #[path]
        pub interaction_id: InteractionId,

        #[path]
        pub token: String,

        #[path]
        pub message_id: InteractionMessageReq,

        #[json]
        pub patch: MessagePatch,

        #[header(rename = "X-Timestamp")]
        pub timestamp: Option<i64>,
    }

    pub struct Response {
        #[json]
        pub message: Message,
    }
}

/// Interaction message delete
///
/// Delete a message from an interaction
#[endpoint(
    delete,
    path = "/interaction/{interaction_id}/{token}/message/{message_id}",
    tags = ["interaction"],
    scopes = [Full],
    response(NO_CONTENT, description = "Delete interaction message success"),
)]
pub mod interaction_message_delete {
    use crate::v1::types::{InteractionId, misc::InteractionMessageReq};

    pub struct Request {
        #[path]
        pub interaction_id: InteractionId,

        #[path]
        pub token: String,

        #[path]
        pub message_id: InteractionMessageReq,
    }

    pub struct Response {}
}
