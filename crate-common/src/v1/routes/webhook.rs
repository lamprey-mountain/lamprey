use lamprey_macros::endpoint;

/// Webhook create
#[endpoint(
    post,
    path = "/channel/{channel_id}/webhook",
    tags = ["webhook"],
    scopes = [Full],
    permissions = [IntegrationsManage],
    audit_log_events = ["WebhookCreate"],
    response(CREATED, body = Webhook, description = "Create webhook success"),
)]
pub mod webhook_create {
    use crate::v1::types::webhook::{Webhook, WebhookCreate};
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub webhook: WebhookCreate,

        #[header]
        pub idempotency_key: Option<String>,
    }

    pub struct Response {
        #[json]
        pub webhook: Webhook,
    }
}

/// Webhook list channel
#[endpoint(
    get,
    path = "/channel/{channel_id}/webhook",
    tags = ["webhook"],
    scopes = [Full],
    permissions = [IntegrationsManage],
    response(OK, body = PaginationResponse<Webhook>, description = "List webhooks success"),
)]
pub mod webhook_list_channel {
    use crate::v1::types::webhook::Webhook;
    use crate::v1::types::WebhookId;
    use crate::v1::types::{ChannelId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<WebhookId>,
    }

    pub struct Response {
        #[json]
        pub webhooks: PaginationResponse<Webhook>,
    }
}

/// Webhook list room
#[endpoint(
    get,
    path = "/room/{room_id}/webhook",
    tags = ["webhook"],
    scopes = [Full],
    permissions = [IntegrationsManage],
    response(OK, body = PaginationResponse<Webhook>, description = "List webhooks success"),
)]
pub mod webhook_list_room {
    use crate::v1::types::webhook::Webhook;
    use crate::v1::types::WebhookId;
    use crate::v1::types::{PaginationQuery, PaginationResponse, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub pagination: PaginationQuery<WebhookId>,
    }

    pub struct Response {
        #[json]
        pub webhooks: PaginationResponse<Webhook>,
    }
}

/// Webhook get
#[endpoint(
    get,
    path = "/webhook/{webhook_id}",
    tags = ["webhook"],
    scopes = [Full],
    permissions = [IntegrationsManage],
    response(OK, body = Webhook, description = "Get webhook success"),
)]
pub mod webhook_get {
    use crate::v1::types::webhook::Webhook;
    use crate::v1::types::WebhookId;

    pub struct Request {
        #[path]
        pub webhook_id: WebhookId,
    }

    pub struct Response {
        #[json]
        pub webhook: Webhook,
    }
}

/// Webhook get with token
#[endpoint(
    get,
    path = "/webhook/{webhook_id}/{token}",
    tags = ["webhook"],
    response(OK, body = Webhook, description = "Get webhook success"),
)]
pub mod webhook_get_with_token {
    use crate::v1::types::webhook::Webhook;
    use crate::v1::types::WebhookId;

    pub struct Request {
        #[path]
        pub webhook_id: WebhookId,

        #[path]
        pub token: String,
    }

    pub struct Response {
        #[json]
        pub webhook: Webhook,
    }
}

/// Webhook delete
#[endpoint(
    delete,
    path = "/webhook/{webhook_id}",
    tags = ["webhook"],
    scopes = [Full],
    permissions = [IntegrationsManage],
    audit_log_events = ["WebhookDelete"],
    response(NO_CONTENT, description = "Delete webhook success"),
)]
pub mod webhook_delete {
    use crate::v1::types::WebhookId;

    pub struct Request {
        #[path]
        pub webhook_id: WebhookId,
    }

    pub struct Response {}
}

/// Webhook delete with token
#[endpoint(
    delete,
    path = "/webhook/{webhook_id}/{token}",
    tags = ["webhook"],
    response(NO_CONTENT, description = "Delete webhook success"),
)]
pub mod webhook_delete_with_token {
    use crate::v1::types::WebhookId;

    pub struct Request {
        #[path]
        pub webhook_id: WebhookId,

        #[path]
        pub token: String,
    }

    pub struct Response {}
}

/// Webhook update
#[endpoint(
    patch,
    path = "/webhook/{webhook_id}",
    tags = ["webhook"],
    scopes = [Full],
    permissions = [IntegrationsManage],
    audit_log_events = ["WebhookUpdate"],
    response(OK, body = Webhook, description = "Update webhook success"),
)]
pub mod webhook_update {
    use crate::v1::types::webhook::{Webhook, WebhookUpdate};
    use crate::v1::types::WebhookId;

    pub struct Request {
        #[path]
        pub webhook_id: WebhookId,

        #[json]
        pub webhook: WebhookUpdate,
    }

    pub struct Response {
        #[json]
        pub webhook: Webhook,
    }
}

/// Webhook update with token
#[endpoint(
    patch,
    path = "/webhook/{webhook_id}/{token}",
    tags = ["webhook"],
    response(OK, body = Webhook, description = "Update webhook success"),
)]
pub mod webhook_update_with_token {
    use crate::v1::types::webhook::{Webhook, WebhookUpdate};
    use crate::v1::types::WebhookId;

    pub struct Request {
        #[path]
        pub webhook_id: WebhookId,

        #[path]
        pub token: String,

        #[json]
        pub webhook: WebhookUpdate,
    }

    pub struct Response {
        #[json]
        pub webhook: Webhook,
    }
}

/// Webhook execute
#[endpoint(
    post,
    path = "/webhook/{webhook_id}/{token}",
    tags = ["webhook"],
    response(CREATED, body = Message, description = "Execute webhook success"),
)]
pub mod webhook_execute {
    use crate::v1::types::{Message, MessageCreate, WebhookId};

    pub struct Request {
        #[path]
        pub webhook_id: WebhookId,

        #[path]
        pub token: String,

        #[json]
        pub message: MessageCreate,
    }

    pub struct Response {
        #[json]
        pub message: Message,
    }
}

/// Webhook message get
#[endpoint(
    get,
    path = "/webhook/{webhook_id}/{token}/message/{message_id}",
    tags = ["webhook"],
    response(OK, body = Message, description = "Get webhook message success"),
)]
pub mod webhook_message_get {
    use crate::v1::types::{Message, MessageId, WebhookId};

    pub struct Request {
        #[path]
        pub webhook_id: WebhookId,

        #[path]
        pub token: String,

        #[path]
        pub message_id: MessageId,
    }

    pub struct Response {
        #[json]
        pub message: Message,
    }
}

/// Webhook message edit
#[endpoint(
    patch,
    path = "/webhook/{webhook_id}/{token}/message/{message_id}",
    tags = ["webhook"],
    response(OK, body = Message, description = "Edit webhook message success"),
)]
pub mod webhook_message_edit {
    use crate::v1::types::{Message, MessageId, WebhookId};
    use crate::v2::types::message::MessagePatch;

    pub struct Request {
        #[path]
        pub webhook_id: WebhookId,

        #[path]
        pub token: String,

        #[path]
        pub message_id: MessageId,

        #[json]
        pub patch: MessagePatch,
    }

    pub struct Response {
        #[json]
        pub message: Message,
    }
}

/// Webhook message delete
#[endpoint(
    delete,
    path = "/webhook/{webhook_id}/{token}/message/{message_id}",
    tags = ["webhook"],
    response(NO_CONTENT, description = "Delete webhook message success"),
)]
pub mod webhook_message_delete {
    use crate::v1::types::{MessageId, WebhookId};

    pub struct Request {
        #[path]
        pub webhook_id: WebhookId,

        #[path]
        pub token: String,

        #[path]
        pub message_id: MessageId,
    }

    pub struct Response {}
}
