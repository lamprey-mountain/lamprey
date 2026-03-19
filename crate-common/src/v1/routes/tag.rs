use lamprey_macros::endpoint;

/// Tag create
#[endpoint(
    post,
    path = "/channel/{channel_id}/tag",
    tags = ["tag"],
    scopes = [Full],
    permissions = [TagManage],
    response(CREATED, body = Tag, description = "Create tag success"),
)]
pub mod tag_create {
    use crate::v1::types::{ChannelId, Tag, TagCreate};

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
    permissions = [TagManage],
    response(OK, body = Tag, description = "Update tag success"),
)]
pub mod tag_update {
    use crate::v1::types::{ChannelId, Tag, TagId, TagPatch};

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
    permissions = [TagManage],
    response(NO_CONTENT, description = "Delete tag success"),
)]
pub mod tag_delete {
    use crate::v1::types::{ChannelId, TagId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub tag_id: TagId,

        #[query]
        pub force: bool,
    }

    pub struct Response;
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
    use crate::v1::types::{ChannelId, PaginationQuery, PaginationResponse, Tag, TagId, TagListQuery};

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
    use crate::v1::types::{ChannelId, Tag, TagId};

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
    use crate::v1::types::{ChannelId, PaginationQuery, PaginationResponse, Tag, TagId, TagSearchQuery};

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
