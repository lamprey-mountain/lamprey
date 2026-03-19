use lamprey_macros::endpoint;

/// Calendar event list user
///
/// List all events the current user can see
#[endpoint(
    get,
    path = "/calendar/event",
    tags = ["calendar"],
    scopes = [Full],
    response(OK, description = "ok"),
)]
pub mod calendar_event_list_user {
    use crate::v1::types::calendar::CalendarEventListQuery;

    pub struct Request {
        #[query]
        pub query: CalendarEventListQuery,
    }

    pub struct Response;
}

/// Calendar event list
#[endpoint(
    get,
    path = "/calendar/{channel_id}/event",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Vec<CalendarEvent>, description = "ok"),
)]
pub mod calendar_event_list {
    use crate::v1::types::calendar::{CalendarEvent, CalendarEventListQuery};
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub query: CalendarEventListQuery,
    }

    pub struct Response {
        #[json]
        pub events: Vec<CalendarEvent>,
    }
}

/// Calendar event create
#[endpoint(
    post,
    path = "/calendar/{channel_id}/event",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [CalendarEventCreate],
    response(CREATED, body = CalendarEvent, description = "Create calendar event success"),
)]
pub mod calendar_event_create {
    use crate::v1::types::calendar::{CalendarEvent, CalendarEventCreate};
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub event: CalendarEventCreate,
    }

    pub struct Response {
        #[json]
        pub event: CalendarEvent,
    }
}

/// Calendar event get
#[endpoint(
    get,
    path = "/calendar/{channel_id}/event/{event_id}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = CalendarEvent, description = "ok"),
)]
pub mod calendar_event_get {
    use crate::v1::types::calendar::CalendarEvent;
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,
    }

    pub struct Response {
        #[json]
        pub event: CalendarEvent,
    }
}

/// Calendar event update
#[endpoint(
    patch,
    path = "/calendar/{channel_id}/event/{event_id}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [CalendarEventEdit],
    response(OK, body = CalendarEvent, description = "Update calendar event success"),
)]
pub mod calendar_event_update {
    use crate::v1::types::calendar::{CalendarEvent, CalendarEventPatch};
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[json]
        pub patch: CalendarEventPatch,
    }

    pub struct Response {
        #[json]
        pub event: CalendarEvent,
    }
}

/// Calendar event delete
#[endpoint(
    delete,
    path = "/calendar/{channel_id}/event/{event_id}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [CalendarEventEdit],
    response(NO_CONTENT, description = "Delete calendar event success"),
)]
pub mod calendar_event_delete {
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,
    }

    pub struct Response;
}

/// Calendar event participant list
#[endpoint(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/participant",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Vec<CalendarEventParticipant>, description = "ok"),
)]
pub mod calendar_event_participant_list {
    use crate::v1::types::calendar::{CalendarEventParticipant, CalendarEventParticipantQuery};
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[query]
        pub query: CalendarEventParticipantQuery,
    }

    pub struct Response {
        #[json]
        pub participants: Vec<CalendarEventParticipant>,
    }
}

/// Calendar event participant add
#[endpoint(
    put,
    path = "/calendar/{channel_id}/event/{event_id}/participant/{user_id}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [CalendarEventEdit],
    response(OK, body = CalendarEventParticipant, description = "ok"),
)]
pub mod calendar_event_participant_add {
    use crate::v1::types::calendar::{CalendarEventParticipant, CalendarEventParticipantPut};
    use crate::v1::types::{CalendarEventId, ChannelId, UserId};
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub participant: CalendarEventParticipantPut,
    }

    pub struct Response {
        #[json]
        pub participant: CalendarEventParticipant,
    }
}

/// Calendar event participant remove
#[endpoint(
    delete,
    path = "/calendar/{channel_id}/event/{event_id}/participant/{user_id}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [CalendarEventEdit],
    response(NO_CONTENT, description = "ok"),
)]
pub mod calendar_event_participant_remove {
    use crate::v1::types::{CalendarEventId, ChannelId, UserId};
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response;
}

/// Calendar overwrite list
#[endpoint(
    get,
    path = "/calendar/{channel_id}/overwrite",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [CalendarEventEdit],
    response(OK, body = Vec<CalendarOverwrite>, description = "ok"),
)]
pub mod calendar_overwrite_list {
    use crate::v1::types::calendar::CalendarOverwrite;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {
        #[json]
        pub overwrites: Vec<CalendarOverwrite>,
    }
}

/// Calendar overwrite put
#[endpoint(
    put,
    path = "/calendar/{channel_id}/overwrite/{user_id}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [CalendarEventEdit],
    response(OK, body = CalendarOverwrite, description = "ok"),
)]
pub mod calendar_overwrite_put {
    use crate::v1::types::calendar::{CalendarOverwrite, CalendarOverwritePut};
    use crate::v1::types::ChannelId;
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub overwrite: CalendarOverwritePut,
    }

    pub struct Response {
        #[json]
        pub overwrite: CalendarOverwrite,
    }
}
