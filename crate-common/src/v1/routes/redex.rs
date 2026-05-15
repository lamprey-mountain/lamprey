use lamprey_macros::endpoint;

/// Redex create
///
/// Create a new redex in a channel
#[endpoint(
    post,
    path = "/channel/{channel_id}/redex",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(CREATED, body = Redex, description = "Create redex success"),
)]
pub mod redex_create {
    use crate::v1::types::{
        redex::{Redex, RedexCreate},
        ChannelId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub redex: RedexCreate,
    }

    pub struct Response {
        #[json]
        pub redex: Redex,
    }
}

/// Redex list
///
/// List scripts in a channel
#[endpoint(
    get,
    path = "/channel/{channel_id}/redex",
    tags = ["redex", "badge.public"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<Redex>, description = "List scripts success"),
)]
pub mod redex_list {
    use crate::v1::types::{redex::Redex, ChannelId, PaginationQuery, PaginationResponse, RedexId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<RedexId>,
    }

    pub struct Response {
        #[json]
        pub scripts: PaginationResponse<Redex>,
    }
}

/// Redex get
///
/// Get a redex by ID
#[endpoint(
    get,
    path = "/channel/{channel_id}/redex/{redex_id}",
    tags = ["redex", "badge.public"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Redex, description = "Get redex success"),
)]
pub mod redex_get {
    use crate::v1::types::{redex::Redex, ChannelId, RedexId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,
    }

    pub struct Response {
        #[json]
        pub redex: Redex,
    }
}

/// Redex delete
///
/// Delete a redex
#[endpoint(
    delete,
    path = "/channel/{channel_id}/redex/{redex_id}",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptManage],
    audit_log_events = ["ScriptDelete"],
    response(NO_CONTENT, description = "Delete redex success"),
)]
pub mod redex_delete {
    use crate::v1::types::{ChannelId, RedexId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,
    }

    pub struct Response {}
}

/// Redex content update
///
/// Update the content of a redex (creates a new version)
#[endpoint(
    put,
    path = "/channel/{channel_id}/redex/{redex_id}/content",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(OK, body = RedexVersion, description = "Update redex content success"),
)]
pub mod redex_content_update {
    use crate::v1::types::{
        redex::{RedexContentUpdate, RedexVersion},
        ChannelId, RedexId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,

        #[json]
        pub content: RedexContentUpdate,
    }

    pub struct Response {
        #[json]
        pub version: RedexVersion,
    }
}

/// Redex trigger
///
/// Run a redex with a trigger input
#[endpoint(
    post,
    path = "/channel/{channel_id}/redex/{redex_id}/trigger",
    tags = ["redex"],
    scopes = [Full],
    response(CREATED, body = Eval, description = "Start redex run success"),
)]
pub mod redex_trigger {
    use crate::v1::types::{
        redex::{Eval, EvalCreateManual},
        ChannelId, RedexId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,

        #[json]
        pub run: EvalCreateManual,
    }

    pub struct Response {
        #[json]
        pub run: Eval,
    }
}

/// Redex version list
///
/// Get version history for a redex
#[endpoint(
    get,
    path = "/channel/{channel_id}/redex/{redex_id}/version",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<RedexVersion>, description = "List redex versions success"),
)]
pub mod redex_version_list {
    use crate::v1::types::{
        redex::RedexVersion, ChannelId, PaginationQuery, PaginationResponse, RedexId, RedexVerId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,

        #[query]
        pub pagination: PaginationQuery<RedexVerId>,
    }

    pub struct Response {
        #[json]
        pub versions: PaginationResponse<RedexVersion>,
    }
}

/// Redex version get
///
/// Get a specific redex version
#[endpoint(
    get,
    path = "/channel/{channel_id}/redex/{redex_id}/version/{version_id}",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = RedexVersion, description = "Get redex version success"),
)]
pub mod redex_version_get {
    use crate::v1::types::{redex::RedexVersion, ChannelId, RedexId, RedexVerId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,

        #[path]
        pub version_id: RedexVerId,
    }

    pub struct Response {
        #[json]
        pub version: RedexVersion,
    }
}

/// Redex version delete
///
/// Delete a specific redex version
#[endpoint(
    delete,
    path = "/channel/{channel_id}/redex/{redex_id}/version/{version_id}",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptManage],
    audit_log_events = ["RedexVersionDelete"],
    response(NO_CONTENT, description = "Delete redex version success"),
)]
pub mod redex_version_delete {
    use crate::v1::types::{ChannelId, RedexId, RedexVerId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,

        #[path]
        pub version_id: RedexVerId,
    }

    pub struct Response {}
}

/// Redex version restore
///
/// Restore a deleted redex version
#[endpoint(
    post,
    path = "/channel/{channel_id}/redex/{redex_id}/version/{version_id}/restore",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(OK, body = RedexVersion, description = "Restore redex version success"),
)]
pub mod redex_version_restore {
    use crate::v1::types::{redex::RedexVersion, ChannelId, RedexId, RedexVerId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,

        #[path]
        pub version_id: RedexVerId,
    }

    pub struct Response {
        #[json]
        pub version: RedexVersion,
    }
}

/// Redex dependency graph
///
/// Get the dependency graph for a redex
#[endpoint(
    get,
    path = "/channel/{channel_id}/redex/{redex_id}/depends",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = RedexDependencyGraph, description = "Get redex dependencies success"),
)]
pub mod redex_depends {
    use crate::v1::types::{redex::RedexDependencyGraph, ChannelId, RedexId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,
    }

    pub struct Response {
        #[json]
        pub dependencies: RedexDependencyGraph,
    }
}

/// Redex dependency update
///
/// Update redex dependencies, creates a new version
#[endpoint(
    post,
    path = "/channel/{channel_id}/redex/{redex_id}/depends/update",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(OK, body = RedexVersion, description = "Update redex dependencies success"),
)]
pub mod redex_depends_update {
    use crate::v1::types::{
        redex::{RedexDependenciesUpdate, RedexVersion},
        ChannelId, RedexId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,

        #[json]
        pub update: RedexDependenciesUpdate,
    }

    pub struct Response {
        #[json]
        pub version: RedexVersion,
    }
}

/// Redex eval list
///
/// List evals for a redex
#[endpoint(
    get,
    path = "/channel/{channel_id}/redex/{redex_id}/eval",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<Eval>, description = "List redex evals success"),
)]
pub mod redex_eval_list {
    use crate::v1::types::{
        redex::Eval, ChannelId, EvalId, PaginationQuery, PaginationResponse, RedexId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,

        #[query]
        pub pagination: PaginationQuery<EvalId>,
    }

    pub struct Response {
        #[json]
        pub evals: PaginationResponse<Eval>,
    }
}

/// Redex eval get
///
/// Get a specific eval
#[endpoint(
    get,
    path = "/channel/{channel_id}/redex/{redex_id}/eval/{eval_id}",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptInspect],
    response(OK, body = Eval, description = "Get redex eval success"),
)]
pub mod redex_eval_get {
    use crate::v1::types::{redex::Eval, ChannelId, EvalId, RedexId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,

        #[path]
        pub eval_id: EvalId,
    }

    pub struct Response {
        #[json]
        pub eval: Eval,
    }
}

/// Redex eval stop
///
/// Stop a evalning redex
#[endpoint(
    post,
    path = "/channel/{channel_id}/redex/{redex_id}/eval/{eval_id}/stop",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(NO_CONTENT, description = "Stop redex eval success"),
)]
pub mod redex_eval_stop {
    use crate::v1::types::{ChannelId, EvalId, RedexId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,

        #[path]
        pub eval_id: EvalId,
    }

    pub struct Response {}
}

/// Redex eval log
///
/// Get logs from a redex eval
#[endpoint(
    get,
    path = "/channel/{channel_id}/redex/{redex_id}/eval/{eval_id}/log",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptInspect],
    response(OK, body = Vec<EvalLogEntry>, description = "Get redex eval logs success"),
)]
pub mod redex_eval_log {
    use crate::v1::types::{
        redex::EvalLogEntry, ChannelId, EvalId, PaginationQuery, PaginationResponse, RedexId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub redex_id: RedexId,

        #[path]
        pub eval_id: EvalId,

        #[query]
        pub pagination: PaginationQuery<u64>,
    }

    pub struct Response {
        #[json]
        pub logs: PaginationResponse<EvalLogEntry>,
    }
}
