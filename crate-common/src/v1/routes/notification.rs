use lamprey_macros::endpoint;

/// Inbox get
///
/// List notifications
#[endpoint(
    get,
    path = "/inbox",
    tags = ["inbox"],
    scopes = [Full],
    response(OK, body = NotificationPagination, description = "success"),
)]
pub mod inbox_get {
    use crate::v1::types::notifications::{InboxListParams, NotificationPagination};
    use crate::v1::types::{NotificationId, PaginationQuery};

    pub struct Request {
        #[query]
        pub pagination: PaginationQuery<NotificationId>,

        #[query]
        pub params: InboxListParams,
    }

    pub struct Response {
        #[json]
        pub inbox: NotificationPagination,
    }
}

/// Inbox post
///
/// Create a reminder for later
#[endpoint(
    post,
    path = "/inbox",
    tags = ["inbox"],
    scopes = [Full],
    response(CREATED, body = Notification, description = "success"),
)]
pub mod inbox_post {
    use crate::v1::types::notifications::{Notification, NotificationCreate};

    pub struct Request {
        #[json]
        pub notification: NotificationCreate,
    }

    pub struct Response {
        #[json]
        pub notification: Notification,
    }
}

/// Inbox mark read
#[endpoint(
    put,
    path = "/inbox/{notification_id}/read",
    tags = ["inbox"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod inbox_mark_read {
    use crate::v1::types::NotificationId;

    pub struct Request {
        #[path]
        pub notification_id: NotificationId,
    }

    pub struct Response;
}

/// Inbox delete
#[endpoint(
    delete,
    path = "/inbox/{notification_id}",
    tags = ["inbox"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod inbox_delete {
    use crate::v1::types::NotificationId;

    pub struct Request {
        #[path]
        pub notification_id: NotificationId,
    }

    pub struct Response;
}

/// Inbox flush
#[endpoint(
    post,
    path = "/inbox/flush",
    tags = ["inbox"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod inbox_flush {
    use crate::v1::types::notifications::NotificationFlush;

    pub struct Request {
        #[json]
        pub flush: NotificationFlush,
    }

    pub struct Response;
}
