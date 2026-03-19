use lamprey_macros::endpoint;

/// Wiki history
///
/// Query edit history for all documents in this wiki
#[endpoint(
    get,
    path = "/wiki/{channel_id}/history",
    tags = ["document"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = HistoryPagination, description = "ok"),
)]
pub mod wiki_history {
    use crate::v1::types::document::{HistoryPagination, HistoryParams};
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub query: HistoryParams,
    }

    pub struct Response {
        #[json]
        pub history: HistoryPagination,
    }
}

/// Document branch list
#[endpoint(
    get,
    path = "/document/{channel_id}/branch",
    tags = ["document"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<DocumentBranch>, description = "ok"),
)]
pub mod document_branch_list {
    use crate::v1::types::document::{DocumentBranch, DocumentBranchListParams};
    use crate::v1::types::{ChannelId, DocumentBranchId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub query: DocumentBranchListParams,

        #[query]
        pub pagination: PaginationQuery<DocumentBranchId>,
    }

    pub struct Response {
        #[json]
        pub branches: PaginationResponse<DocumentBranch>,
    }
}

/// Document branch get
#[endpoint(
    get,
    path = "/document/{channel_id}/branch/{branch_id}",
    tags = ["document"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = DocumentBranch, description = "ok"),
)]
pub mod document_branch_get {
    use crate::v1::types::document::DocumentBranch;
    use crate::v1::types::{ChannelId, DocumentBranchId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub branch_id: DocumentBranchId,
    }

    pub struct Response {
        #[json]
        pub branch: DocumentBranch,
    }
}

/// Document branch create
#[endpoint(
    post,
    path = "/document/{channel_id}/branch",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(CREATED, body = DocumentBranch, description = "ok"),
)]
pub mod document_branch_create {
    use crate::v1::types::document::{DocumentBranch, DocumentBranchCreate};
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub branch: DocumentBranchCreate,
    }

    pub struct Response {
        #[json]
        pub branch: DocumentBranch,
    }
}

/// Document branch patch
#[endpoint(
    patch,
    path = "/document/{channel_id}/branch/{branch_id}",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(OK, body = DocumentBranch, description = "ok"),
)]
pub mod document_branch_patch {
    use crate::v1::types::document::{DocumentBranch, DocumentBranchPatch};
    use crate::v1::types::{ChannelId, DocumentBranchId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub branch_id: DocumentBranchId,

        #[json]
        pub patch: DocumentBranchPatch,
    }

    pub struct Response {
        #[json]
        pub branch: DocumentBranch,
    }
}

/// Document branch delete
#[endpoint(
    delete,
    path = "/document/{channel_id}/branch/{branch_id}",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(NO_CONTENT, description = "ok"),
)]
pub mod document_branch_delete {
    use crate::v1::types::{ChannelId, DocumentBranchId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub branch_id: DocumentBranchId,
    }

    pub struct Response;
}

/// Document branch merge
#[endpoint(
    post,
    path = "/document/{channel_id}/branch/{branch_id}/merge",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(OK, description = "ok"),
)]
pub mod document_branch_merge {
    use crate::v1::types::document::DocumentBranchMerge;
    use crate::v1::types::{ChannelId, DocumentBranchId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub branch_id: DocumentBranchId,

        #[json]
        pub merge: DocumentBranchMerge,
    }

    pub struct Response;
}

/// Document CRDT diff
#[endpoint(
    get,
    path = "/document/{channel_id}/branch/{branch_id}/diff",
    tags = ["document"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Serdoc, description = "ok"),
)]
pub mod document_crdt_diff {
    use crate::v1::types::document::serialized::Serdoc;
    use crate::v1::types::document::DocumentCrdtDiffParams;
    use crate::v1::types::{ChannelId, DocumentBranchId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub branch_id: DocumentBranchId,

        #[query]
        pub params: DocumentCrdtDiffParams,
    }

    pub struct Response {
        #[json]
        pub diff: Serdoc,
    }
}

/// Document tag create
#[endpoint(
    post,
    path = "/document/{channel_id}/tag",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(CREATED, description = "ok"),
)]
pub mod document_tag_create {
    use crate::v1::types::document::DocumentTagCreate;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub tag: DocumentTagCreate,
    }

    pub struct Response;
}

/// Document tag patch
#[endpoint(
    patch,
    path = "/document/{channel_id}/tag/{tag_id}",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(OK, description = "ok"),
)]
pub mod document_tag_patch {
    use crate::v1::types::document::DocumentTagPatch;
    use crate::v1::types::{ChannelId, DocumentTagId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub tag_id: DocumentTagId,

        #[json]
        pub patch: DocumentTagPatch,
    }

    pub struct Response;
}

/// Document tag delete
#[endpoint(
    delete,
    path = "/document/{channel_id}/tag/{tag_id}",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(NO_CONTENT, description = "ok"),
)]
pub mod document_tag_delete {
    use crate::v1::types::{ChannelId, DocumentTagId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub tag_id: DocumentTagId,
    }

    pub struct Response;
}
