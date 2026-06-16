use lamprey_macros::endpoint;

/// Tag create
#[endpoint(
    post,
    path = "/channel/{channel_id}/tag",
    tags = ["tag"],
    scopes = [Full],
    permissions = [ChannelEdit],
    audit_log_events = ["TagCreate"],
    response(CREATED, body = Tag, description = "Create tag success"),
)]
pub mod tag_create {
    use crate::v1::types::ChannelId;
    use crate::v1::types::tag::{Tag, TagCreate};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub tag: TagCreate,

        #[header]
        pub idempotency_key: Option<String>,
    }

    pub struct Response {
        #[json]
        pub tag: Tag,
    }
}

/// Tag update
#[endpoint(
    patch,
    path = "/channel/{channel_id}/tag/{tag_id}",
    tags = ["tag"],
    scopes = [Full],
    permissions = [ChannelEdit],
    audit_log_events = ["TagUpdate"],
    response(OK, body = Tag, description = "Update tag success"),
)]
pub mod tag_update {
    use crate::v1::types::tag::{Tag, TagPatch};
    use crate::v1::types::{ChannelId, TagId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub tag_id: TagId,

        #[json]
        pub patch: TagPatch,
    }

    pub struct Response {
        #[json]
        pub tag: Tag,
    }
}

/// Tag delete
#[endpoint(
    delete,
    path = "/channel/{channel_id}/tag/{tag_id}",
    tags = ["tag"],
    scopes = [Full],
    permissions = [ChannelEdit],
    audit_log_events = ["TagDelete"],
    response(NO_CONTENT, description = "Delete tag success"),
)]
pub mod tag_delete {
    use crate::v1::types::tag::TagDeleteQuery;
    use crate::v1::types::{ChannelId, TagId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub tag_id: TagId,

        #[query]
        pub query: TagDeleteQuery,
    }

    pub struct Response {}
}

/// Tag list
#[endpoint(
    get,
    path = "/channel/{channel_id}/tag",
    tags = ["tag"],
    scopes = [Full],
    response(OK, body = PaginationResponse<Tag>, description = "List tags success"),
)]
pub mod tag_list {
    use crate::v1::types::tag::{Tag, TagListQuery};
    use crate::v1::types::{ChannelId, PaginationQuery, PaginationResponse, TagId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub list: TagListQuery,

        #[query]
        pub pagination: PaginationQuery<TagId>,
    }

    pub struct Response {
        #[json]
        pub tags: PaginationResponse<Tag>,
    }
}

/// Tag get
#[endpoint(
    get,
    path = "/channel/{channel_id}/tag/{tag_id}",
    tags = ["tag"],
    scopes = [Full],
    response(OK, body = Tag, description = "Get tag success"),
)]
pub mod tag_get {
    use crate::v1::types::tag::Tag;
    use crate::v1::types::{ChannelId, TagId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub tag_id: TagId,
    }

    pub struct Response {
        #[json]
        pub tag: Tag,
    }
}

/// Tag search
#[endpoint(
    get,
    path = "/channel/{channel_id}/tag/search",
    tags = ["tag"],
    scopes = [Full],
    response(OK, body = PaginationResponse<Tag>, description = "Search tags success"),
)]
pub mod tag_search {
    use crate::v1::types::tag::{Tag, TagSearchQuery};
    use crate::v1::types::{ChannelId, PaginationQuery, PaginationResponse, TagId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub search: TagSearchQuery,

        #[query]
        pub pagination: PaginationQuery<TagId>,
    }

    pub struct Response {
        #[json]
        pub tags: PaginationResponse<Tag>,
    }
}

// TODO: implement tag_query
/// Tag query
///
/// Query a list of ids to full Tag objects
#[endpoint(
    post,
    path = "/channel/{channel_id}/tag/query",
    tags = ["tag"],
    scopes = [Full],
    response(OK, body = ResponseBody, description = "Query tags success"),
)]
pub mod tag_query {
    use crate::v1::types::tag::Tag;
    use crate::v1::types::{ChannelId, TagId};

    #[cfg(feature = "serde")]
    use serde::{Deserialize, Serialize};

    #[cfg(feature = "utoipa")]
    use utoipa::ToSchema;

    #[cfg(feature = "validator")]
    use validator::Validate;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub body: RequestBody,
    }

    pub struct Response {
        #[json]
        pub body: ResponseBody,
    }

    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "utoipa", derive(ToSchema))]
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct RequestBody {
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
        pub tag_ids: Vec<TagId>,
    }

    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "utoipa", derive(ToSchema))]
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct ResponseBody {
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
        pub tags: Vec<Tag>,
    }
}
