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
    use crate::{
        v1::types::{redex::RedexCreate, ChannelId},
        v2::types::redex::Redex,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub redex: RedexCreate,
    }

    pub struct Response {
        #[json]
        pub body: Redex,
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
    use crate::{
        v1::types::{ChannelId, PaginationQuery, PaginationResponse, RedexId},
        v2::types::redex::Redex,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<RedexId>,
    }

    pub struct Response {
        #[json]
        pub body: PaginationResponse<Redex>,
    }
}

/// Redex get
///
/// Get a redex by ID
#[endpoint(
    get,
    path = "/redex/{redex_id}",
    tags = ["redex", "badge.public"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Redex, description = "Get redex success"),
)]
pub mod redex_get {
    use crate::v2::types::{redex::Redex, RedexId};

    pub struct Request {
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
    path = "/redex/{redex_id}",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptManage],
    audit_log_events = ["ScriptDelete"],
    response(NO_CONTENT, description = "Delete redex success"),
)]
pub mod redex_delete {
    use crate::v1::types::RedexId;

    pub struct Request {
        #[path]
        pub redex_id: RedexId,
    }

    pub struct Response {}
}

/// Redex deploy
#[endpoint(
    post,
    path = "/redex/{redex_id}/deploy",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(OK, body = Redex, description = "Deploy redex success"),
)]
pub mod redex_deploy {
    use crate::v1::types::RedexId;
    use crate::v2::types::redex::Redex;

    pub struct Request {
        #[path]
        pub redex_id: RedexId,
    }

    pub struct Response {
        #[json]
        pub body: Redex,
    }
}

/// Redex trigger
///
/// Run a redex with a trigger input
#[endpoint(
    post,
    path = "/redex/{redex_id}/trigger",
    tags = ["redex"],
    scopes = [Full],
    response(CREATED, body = Eval, description = "Start redex run success"),
)]
pub mod redex_trigger {
    use crate::v1::types::{
        redex::{Eval, EvalCreateManual},
        RedexId,
    };

    pub struct Request {
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

/// Redex dependency graph
///
/// Get the dependency graph for a redex
#[cfg(feature = "feat_redex_dependency_graph")]
#[endpoint(
    get,
    path = "/redex/{redex_id}/depends",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = RedexDependencyGraph, description = "Get redex dependencies success"),
)]
pub mod redex_depends {
    use crate::v1::types::RedexId;
    use crate::v2::types::redex::DependencyGraph;

    pub struct Request {
        #[path]
        pub redex_id: RedexId,
    }

    pub struct Response {
        #[json]
        pub dependencies: DependencyGraph,
    }
}

/// Redex dependency update
///
/// Update redex dependencies, creates a new version
#[cfg(feature = "feat_redex_dependency_graph")]
#[endpoint(
    post,
    path = "/redex/{redex_id}/depends/update",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(OK, body = DependencyGraph, description = "Update redex dependencies success"),
)]
pub mod redex_depends_update {
    use crate::v1::types::RedexId;
    use crate::v2::types::redex::{DependencyGraph, DependencyUpdateRequest};

    pub struct Request {
        #[path]
        pub redex_id: RedexId,

        #[json]
        pub update: DependencyUpdateRequest,
    }

    pub struct Response {
        #[json]
        pub body: DependencyGraph,
    }
}

/// Redex eval list
///
/// List evals for a redex
#[endpoint(
    get,
    path = "/redex/{redex_id}/eval",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<Eval>, description = "List redex evals success"),
)]
pub mod redex_eval_list {
    use crate::v1::types::{redex::Eval, EvalId, PaginationQuery, PaginationResponse, RedexId};

    pub struct Request {
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
    path = "/redex/{redex_id}/eval/{eval_id}",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptInspect],
    response(OK, body = Eval, description = "Get redex eval success"),
)]
pub mod redex_eval_get {
    use crate::v1::types::{redex::Eval, EvalId, RedexId};

    pub struct Request {
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
/// Stop an eval
#[endpoint(
    post,
    path = "/redex/{redex_id}/eval/{eval_id}/stop",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(NO_CONTENT, description = "Stop redex eval success"),
)]
pub mod redex_eval_stop {
    use crate::v1::types::{EvalId, RedexId};

    pub struct Request {
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
    path = "/redex/{redex_id}/eval/{eval_id}/log",
    tags = ["redex"],
    scopes = [Full],
    permissions = [ScriptInspect],
    response(OK, body = Vec<EvalLogEntry>, description = "Get redex eval logs success"),
)]
pub mod redex_eval_log {
    use crate::v1::types::{
        redex::EvalLogEntry, EvalId, PaginationQuery, PaginationResponse, RedexId,
    };

    pub struct Request {
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
