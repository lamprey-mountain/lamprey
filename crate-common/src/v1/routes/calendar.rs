use lamprey_macros::endpoint;

/// Calendar event list user
///
/// List all events the current user can see
#[endpoint(
    get,
    path = "/calendar/event",
    tags = ["calendar"],
    scopes = [Full],
    response(OK, body = CalendarEventList, description = "ok"),
)]
pub mod calendar_event_list_user {
    use crate::v1::types::calendar::{CalendarEventList, CalendarEventListQuery};

    pub struct Request {
        #[query]
        pub query: CalendarEventListQuery,
    }

    pub struct Response {
        #[json]
        pub body: CalendarEventList,
    }
}

/// Calendar export ics
#[endpoint(
    get,
    path = "/calendar/{channel_id}/feed.ics",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = String, description = "ok"),
)]
pub mod calendar_export {
    use crate::util::body::Body;
    use crate::v1::types::ChannelId;
    use crate::v1::types::calendar::{CalendarEventListQuery, CalendarExportQuery};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub export_query: CalendarExportQuery,

        #[query]
        pub query: CalendarEventListQuery,
    }

    pub struct Response {
        /// always `Content-Type: text/calendar; charset=utf-8`
        #[header]
        pub content_type: String,

        // TODO: support ETag/Last-Modified
        #[body]
        pub body: Body,
    }
}

/// Calendar event list
#[endpoint(
    get,
    path = "/calendar/{channel_id}/event",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = CalendarEventList, description = "ok"),
)]
pub mod calendar_event_list {
    use crate::v1::types::ChannelId;
    use crate::v1::types::calendar::{CalendarEventList, CalendarEventListQuery};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub query: CalendarEventListQuery,
    }

    pub struct Response {
        #[json]
        pub body: CalendarEventList,
    }
}

/// Calendar event create
#[endpoint(
    post,
    path = "/calendar/{channel_id}/event",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [CalendarEventCreate],
    audit_log_events = ["CalendarEventCreate"],
    response(CREATED, body = CalendarEvent, description = "Create calendar event success"),
)]
pub mod calendar_event_create {
    use crate::v1::types::ChannelId;
    use crate::v1::types::calendar::{CalendarEvent, CalendarEventCreate};

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
    permissions = [CalendarEventManage],
    audit_log_events = ["CalendarEventUpdate"],
    response(OK, body = CalendarEvent, description = "Update calendar event success"),
)]
pub mod calendar_event_update {
    use crate::v1::types::calendar::{CalendarEvent, CalendarEventUpdate};
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[json]
        pub patch: CalendarEventUpdate,
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
    permissions = [CalendarEventManage],
    audit_log_events = ["CalendarEventDelete"],
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

    pub struct Response {}
}

/// Calendar instance list
#[endpoint(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/instance",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Vec<CalendarInstance>, description = "List calendar instances success"),
)]
pub mod calendar_instance_list {
    use crate::v1::types::calendar::CalendarInstance;
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,
    }

    pub struct Response {
        #[json]
        pub instances: Vec<CalendarInstance>,
    }
}

/// Calendar instance get
#[endpoint(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/instance/{seq}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = CalendarInstance, description = "Get calendar instance success"),
)]
pub mod calendar_instance_get {
    use crate::v1::types::calendar::{CalendarInstance, CalendarInstanceSeq};
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub seq: CalendarInstanceSeq,
    }

    pub struct Response {
        #[json]
        pub instance: CalendarInstance,
    }
}

/// Calendar event RSVP invite
#[endpoint(
    post,
    path = "/calendar/{channel_id}/event/{event_id}/rsvp",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [CalendarEventManage],
    audit_log_events = ["CalendarRsvpCreate"],
    response(CREATED, description = "Invite users to calendar event success"),
)]
pub mod calendar_rsvp_invite {
    use crate::v1::types::calendar::CalendarParticipantInvite;
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[json]
        pub invite: CalendarParticipantInvite,
    }

    pub struct Response {}
}

/// Calendar event RSVP list
#[endpoint(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/rsvp",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Vec<CalendarParticipant>, description = "ok"),
)]
pub mod calendar_event_rsvp_list {
    use crate::v1::types::calendar::{CalendarParticipant, CalendarParticipantQuery};
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[query]
        pub query: CalendarParticipantQuery,
    }

    pub struct Response {
        #[json]
        pub participants: Vec<CalendarParticipant>,
    }
}

/// Calendar event RSVP get
#[endpoint(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/rsvp/{user_id}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = CalendarParticipant, description = "ok"),
)]
pub mod calendar_event_rsvp_get {
    use crate::v1::types::calendar::CalendarParticipant;
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {
        #[json]
        pub participant: CalendarParticipant,
    }
}

/// Calendar event RSVP put
#[endpoint(
    put,
    path = "/calendar/{channel_id}/event/{event_id}/rsvp/{user_id}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelEdit],
    // audit_log_events = ["CalendarRsvpDelete"], // NOTE: adding other users should probably audit log
    response(OK, description = "ok"),
)]
pub mod calendar_event_rsvp_put {
    use crate::v1::types::calendar::CalendarParticipantPut;
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub participant: CalendarParticipantPut,
    }

    pub struct Response {}
}

/// Calendar event RSVP delete
#[endpoint(
    delete,
    path = "/calendar/{channel_id}/event/{event_id}/rsvp/{user_id}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelEdit],
    audit_log_events = ["CalendarRsvpDelete"],
    response(NO_CONTENT, description = "Delete calendar event RSVP success"),
)]
pub mod calendar_event_rsvp_delete {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {}
}

/// Calendar overwrite list
#[endpoint(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Vec<CalendarOverwrite>, description = "List calendar overwrites success"),
)]
pub mod calendar_overwrite_list {
    use crate::v1::types::calendar::CalendarOverwrite;
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,
    }

    pub struct Response {
        #[json]
        pub overwrites: Vec<CalendarOverwrite>,
    }
}

/// Calendar overwrite get
#[endpoint(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite/{seq}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = CalendarOverwrite, description = "Get calendar overwrite success"),
)]
pub mod calendar_overwrite_get {
    use crate::v1::types::calendar::{CalendarInstanceSeq, CalendarOverwrite};
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub seq: CalendarInstanceSeq,
    }

    pub struct Response {
        #[json]
        pub overwrite: CalendarOverwrite,
    }
}

/// Calendar overwrite put
///
/// Create or replace an overwrite for a calendar event
#[endpoint(
    put,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite/{seq}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [CalendarEventManage],
    audit_log_events = ["CalendarOverwriteUpdate"],
    response(OK, body = CalendarOverwrite, description = "Put calendar overwrite success"),
)]
pub mod calendar_overwrite_update {
    use crate::v1::types::calendar::{CalendarInstanceSeq, CalendarOverwrite};
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub seq: CalendarInstanceSeq,

        #[json]
        pub overwrite: CalendarOverwrite,
    }

    pub struct Response {
        #[json]
        pub overwrite: CalendarOverwrite,
    }
}

/// Calendar overwrite delete
#[endpoint(
    delete,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite/{seq}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [CalendarEventManage],
    audit_log_events = ["CalendarOverwriteDelete"],
    response(NO_CONTENT, description = "Delete calendar overwrite success"),
)]
pub mod calendar_overwrite_delete {
    use crate::v1::types::{CalendarEventId, ChannelId, calendar::CalendarInstanceSeq};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub seq: CalendarInstanceSeq,
    }

    pub struct Response {}
}

/// Calendar instance RSVP list
#[endpoint(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/instance/{seq}/rsvp",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Vec<CalendarParticipant>, description = "ok"),
)]
pub mod calendar_instance_rsvp_list {
    use crate::v1::types::calendar::{
        CalendarInstanceSeq, CalendarParticipant, CalendarParticipantQuery,
    };
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub seq: CalendarInstanceSeq,

        #[query]
        pub query: CalendarParticipantQuery,
    }

    pub struct Response {
        #[json]
        pub participants: Vec<CalendarParticipant>,
    }
}

/// Calendar instance RSVP invite
#[endpoint(
    post,
    path = "/calendar/{channel_id}/event/{event_id}/instance/{seq}/rsvp",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelEdit],
    audit_log_events = ["CalendarRsvpCreate"],
    response(CREATED, description = "Invite users to calendar instance success"),
)]
pub mod calendar_instance_rsvp_invite {
    use crate::v1::types::calendar::{CalendarInstanceSeq, CalendarParticipantInvite};
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub seq: CalendarInstanceSeq,

        #[json]
        pub invite: CalendarParticipantInvite,
    }

    pub struct Response {}
}

/// Calendar instance RSVP get
#[endpoint(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/instance/{seq}/rsvp/{user_id}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = CalendarParticipant, description = "ok"),
)]
pub mod calendar_instance_rsvp_get {
    use crate::v1::types::calendar::{CalendarInstanceSeq, CalendarParticipant};
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub seq: CalendarInstanceSeq,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {
        #[json]
        pub participant: CalendarParticipant,
    }
}

/// Calendar instance RSVP put
#[endpoint(
    put,
    path = "/calendar/{channel_id}/event/{event_id}/instance/{seq}/rsvp/{user_id}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelEdit],
    audit_log_events = ["CalendarRsvpUpdate"],
    response(OK, description = "ok"),
)]
pub mod calendar_instance_rsvp_put {
    use crate::v1::types::calendar::{
        CalendarInstanceSeq, CalendarParticipant, CalendarParticipantPut,
    };
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub seq: CalendarInstanceSeq,

        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub participant: CalendarParticipantPut,
    }

    pub struct Response {
        #[json]
        pub participant: CalendarParticipant,
    }
}

/// Calendar instance RSVP delete
#[endpoint(
    delete,
    path = "/calendar/{channel_id}/event/{event_id}/instance/{seq}/rsvp/{user_id}",
    tags = ["calendar"],
    scopes = [Full],
    permissions = [ChannelEdit],
    audit_log_events = ["CalendarRsvpDelete"],
    response(NO_CONTENT, description = "Delete calendar instance RSVP success"),
)]
pub mod calendar_instance_rsvp_delete {
    use crate::v1::types::calendar::CalendarInstanceSeq;
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{CalendarEventId, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub event_id: CalendarEventId,

        #[path]
        pub seq: CalendarInstanceSeq,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {}
}
