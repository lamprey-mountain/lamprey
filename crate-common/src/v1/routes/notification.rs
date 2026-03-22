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
    post,
    path = "/inbox/mark-read",
    tags = ["inbox"],
    scopes = [Full],
    response(OK, description = "success"),
)]
pub mod inbox_mark_read {
    use crate::v1::types::notifications::NotificationMarkRead;

    pub struct Request {
        #[json]
        pub mark_read: NotificationMarkRead,
    }

    pub struct Response {}
}

/// Inbox mark unread
#[endpoint(
    post,
    path = "/inbox/mark-unread",
    tags = ["inbox"],
    scopes = [Full],
    response(OK, description = "success"),
)]
pub mod inbox_mark_unread {
    use crate::v1::types::notifications::NotificationMarkRead;

    pub struct Request {
        #[json]
        pub mark_unread: NotificationMarkRead,
    }

    pub struct Response {}
}

/// Inbox flush
///
/// Deletes read notifications from the inbox
#[endpoint(
    post,
    path = "/inbox/flush",
    tags = ["inbox"],
    scopes = [Full],
    response(OK, description = "success"),
)]
pub mod inbox_flush {
    use crate::v1::types::notifications::NotificationFlush;

    pub struct Request {
        #[json]
        pub flush: NotificationFlush,
    }

    pub struct Response {}
}
