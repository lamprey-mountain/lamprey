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

/// Document branch update
#[endpoint(
    patch,
    path = "/document/{channel_id}/branch/{branch_id}",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(OK, body = DocumentBranch, description = "ok"),
)]
pub mod document_branch_update {
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

/// Document branch close
#[endpoint(
    post,
    path = "/document/{channel_id}/branch/{branch_id}/close",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(OK, body = DocumentBranch, description = "ok"),
)]
pub mod document_branch_close {
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

/// Document branch fork
#[endpoint(
    post,
    path = "/document/{channel_id}/branch/{parent_id}/fork",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(OK, body = DocumentBranch, description = "ok"),
)]
pub mod document_branch_fork {
    use crate::v1::types::document::{DocumentBranch, DocumentBranchCreate};
    use crate::v1::types::{ChannelId, DocumentBranchId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub parent_id: DocumentBranchId,

        #[json]
        pub branch: DocumentBranchCreate,
    }

    pub struct Response {
        #[json]
        pub branch: DocumentBranch,
    }
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

    pub struct Response {}
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

    pub struct Response {}
}

/// Document tag list
#[endpoint(
    get,
    path = "/document/{channel_id}/tag",
    tags = ["document"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Vec<DocumentTag>, description = "ok"),
)]
pub mod document_tag_list {
    use crate::v1::types::document::DocumentTag;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {
        #[json]
        pub tags: Vec<DocumentTag>,
    }
}

/// Document tag get
#[endpoint(
    get,
    path = "/document/{channel_id}/tag/{tag_id}",
    tags = ["document"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = DocumentTag, description = "ok"),
)]
pub mod document_tag_get {
    use crate::v1::types::document::DocumentTag;
    use crate::v1::types::{ChannelId, DocumentTagId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub tag_id: DocumentTagId,
    }

    pub struct Response {
        #[json]
        pub tag: DocumentTag,
    }
}

/// Document tag update
#[endpoint(
    patch,
    path = "/document/{channel_id}/tag/{tag_id}",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(OK, body = DocumentTag, description = "ok"),
)]
pub mod document_tag_update {
    use crate::v1::types::document::{DocumentTag, DocumentTagPatch};
    use crate::v1::types::{ChannelId, DocumentTagId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub tag_id: DocumentTagId,

        #[json]
        pub tag: DocumentTagPatch,
    }

    pub struct Response {
        #[json]
        pub tag: DocumentTag,
    }
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

    pub struct Response {}
}

/// Document history
///
/// Query edit history for a document
#[endpoint(
    get,
    path = "/document/{channel_id}/branch/{branch_id}/history",
    tags = ["document"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = HistoryPagination, description = "ok"),
)]
pub mod document_history {
    use crate::v1::types::document::{HistoryPagination, HistoryParams};
    use crate::v1::types::{ChannelId, DocumentBranchId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub branch_id: DocumentBranchId,

        #[query]
        pub query: HistoryParams,
    }

    pub struct Response {
        #[json]
        pub history: HistoryPagination,
    }
}

/// Document CRDT diff
#[endpoint(
    get,
    path = "/document/{channel_id}/branch/{branch_id}/crdt",
    tags = ["document"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, description = "ok"),
)]
pub mod document_crdt_diff {
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

    pub struct Response {}
}

/// Document CRDT apply
/// Note: Uses base64-encoded update in JSON body since raw binary isn't supported
#[endpoint(
    patch,
    path = "/document/{channel_id}/branch/{branch_id}/crdt",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(NO_CONTENT, description = "ok"),
)]
pub mod document_crdt_apply {
    use crate::v1::types::{ChannelId, DocumentBranchId};
    use bytes::Bytes;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub branch_id: DocumentBranchId,

        #[body]
        pub data: Bytes,
    }

    pub struct Response {}
}

/// Document content get
#[endpoint(
    get,
    path = "/document/{channel_id}/revision/{revision_id}/content",
    tags = ["document"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Serdoc, description = "ok"),
)]
pub mod document_content_get {
    use crate::v1::types::document::serialized::Serdoc;
    use crate::v1::types::document::DocumentRevisionId;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub revision_id: DocumentRevisionId,
    }

    pub struct Response {
        #[json]
        pub content: Serdoc,
    }
}

/// Document content put
#[endpoint(
    put,
    path = "/document/{channel_id}/branch/{branch_id}/content",
    tags = ["document"],
    scopes = [Full],
    permissions = [DocumentEdit],
    response(NO_CONTENT, description = "ok"),
)]
pub mod document_content_put {
    use crate::v1::types::document::SerdocPut;
    use crate::v1::types::{ChannelId, DocumentBranchId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub branch_id: DocumentBranchId,

        #[json]
        pub content: SerdocPut,
    }

    pub struct Response {}
}
