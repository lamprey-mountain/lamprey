use lamprey_macros::endpoint;

/// Script create
///
/// Create a new script in a channel
#[endpoint(
    post,
    path = "/channel/{channel_id}/script",
    tags = ["script"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(CREATED, body = Script, description = "Create script success"),
)]
pub mod script_create {
    use crate::v1::types::{
        script::{Script, ScriptCreate},
        ChannelId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub script: ScriptCreate,
    }

    pub struct Response {
        #[json]
        pub script: Script,
    }
}

/// Script list
///
/// List scripts in a channel
#[endpoint(
    get,
    path = "/channel/{channel_id}/script",
    tags = ["script", "badge.public"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<Script>, description = "List scripts success"),
)]
pub mod script_list {
    use crate::v1::types::{
        script::Script, ChannelId, PaginationQuery, PaginationResponse, ScriptId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<ScriptId>,
    }

    pub struct Response {
        #[json]
        pub scripts: PaginationResponse<Script>,
    }
}

/// Script get
///
/// Get a script by ID
#[endpoint(
    get,
    path = "/channel/{channel_id}/script/{script_id}",
    tags = ["script", "badge.public"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Script, description = "Get script success"),
)]
pub mod script_get {
    use crate::v1::types::{script::Script, ChannelId, ScriptId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,
    }

    pub struct Response {
        #[json]
        pub script: Script,
    }
}

/// Script delete
///
/// Delete a script
#[endpoint(
    delete,
    path = "/channel/{channel_id}/script/{script_id}",
    tags = ["script"],
    scopes = [Full],
    permissions = [ScriptManage],
    audit_log_events = ["ScriptDelete"],
    response(NO_CONTENT, description = "Delete script success"),
)]
pub mod script_delete {
    use crate::v1::types::{ChannelId, ScriptId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,
    }

    pub struct Response {}
}

/// Script content update
///
/// Update the content of a script (creates a new version)
#[endpoint(
    put,
    path = "/channel/{channel_id}/script/{script_id}/content",
    tags = ["script"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(OK, body = ScriptVersion, description = "Update script content success"),
)]
pub mod script_content_update {
    use crate::v1::types::{
        script::{ScriptContentUpdate, ScriptVersion},
        ChannelId, ScriptId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,

        #[json]
        pub content: ScriptContentUpdate,
    }

    pub struct Response {
        #[json]
        pub version: ScriptVersion,
    }
}

/// Script trigger
///
/// Run a script with a trigger input
#[endpoint(
    post,
    path = "/channel/{channel_id}/script/{script_id}/trigger",
    tags = ["script"],
    scopes = [Full],
    response(CREATED, body = Run, description = "Start script run success"),
)]
pub mod script_trigger {
    use crate::v1::types::{
        script::{Run, RunCreateTrigger},
        ChannelId, ScriptId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,

        #[json]
        pub run: RunCreateTrigger,
    }

    pub struct Response {
        #[json]
        pub run: Run,
    }
}

/// Script version list
///
/// Get version history for a script
#[endpoint(
    get,
    path = "/channel/{channel_id}/script/{script_id}/version",
    tags = ["script"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<ScriptVersion>, description = "List script versions success"),
)]
pub mod script_version_list {
    use crate::v1::types::{
        script::ScriptVersion, ChannelId, PaginationQuery, PaginationResponse, ScriptId,
        ScriptVerId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,

        #[query]
        pub pagination: PaginationQuery<ScriptVerId>,
    }

    pub struct Response {
        #[json]
        pub versions: PaginationResponse<ScriptVersion>,
    }
}

/// Script version get
///
/// Get a specific script version
#[endpoint(
    get,
    path = "/channel/{channel_id}/script/{script_id}/version/{version_id}",
    tags = ["script"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = ScriptVersion, description = "Get script version success"),
)]
pub mod script_version_get {
    use crate::v1::types::{script::ScriptVersion, ChannelId, ScriptId, ScriptVerId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,

        #[path]
        pub version_id: ScriptVerId,
    }

    pub struct Response {
        #[json]
        pub version: ScriptVersion,
    }
}

/// Script version delete
///
/// Delete a specific script version
#[endpoint(
    delete,
    path = "/channel/{channel_id}/script/{script_id}/version/{version_id}",
    tags = ["script"],
    scopes = [Full],
    permissions = [ScriptManage],
    audit_log_events = ["ScriptVersionDelete"],
    response(NO_CONTENT, description = "Delete script version success"),
)]
pub mod script_version_delete {
    use crate::v1::types::{ChannelId, ScriptId, ScriptVerId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,

        #[path]
        pub version_id: ScriptVerId,
    }

    pub struct Response {}
}

/// Script version restore
///
/// Restore a deleted script version
#[endpoint(
    post,
    path = "/channel/{channel_id}/script/{script_id}/version/{version_id}/restore",
    tags = ["script"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(OK, body = ScriptVersion, description = "Restore script version success"),
)]
pub mod script_version_restore {
    use crate::v1::types::{script::ScriptVersion, ChannelId, ScriptId, ScriptVerId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,

        #[path]
        pub version_id: ScriptVerId,
    }

    pub struct Response {
        #[json]
        pub version: ScriptVersion,
    }
}

/// Script dependency graph
///
/// Get the dependency graph for a script
#[endpoint(
    get,
    path = "/channel/{channel_id}/script/{script_id}/depends",
    tags = ["script"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = ScriptDependencyGraph, description = "Get script dependencies success"),
)]
pub mod script_depends {
    use crate::v1::types::{script::ScriptDependencyGraph, ChannelId, ScriptId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,
    }

    pub struct Response {
        #[json]
        pub dependencies: ScriptDependencyGraph,
    }
}

/// Script dependency update
///
/// Update script dependencies, creates a new version
#[endpoint(
    post,
    path = "/channel/{channel_id}/script/{script_id}/depends/update",
    tags = ["script"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(OK, body = ScriptVersion, description = "Update script dependencies success"),
)]
pub mod script_depends_update {
    use crate::v1::types::{
        script::{ScriptDependenciesUpdate, ScriptVersion},
        ChannelId, ScriptId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,

        #[json]
        pub update: ScriptDependenciesUpdate,
    }

    pub struct Response {
        #[json]
        pub version: ScriptVersion,
    }
}

/// Script run list
///
/// List runs for a script
#[endpoint(
    get,
    path = "/channel/{channel_id}/script/{script_id}/run",
    tags = ["script"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<Run>, description = "List script runs success"),
)]
pub mod script_run_list {
    use crate::v1::types::{
        script::Run, ChannelId, PaginationQuery, PaginationResponse, RunId, ScriptId,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,

        #[query]
        pub pagination: PaginationQuery<RunId>,
    }

    pub struct Response {
        #[json]
        pub runs: PaginationResponse<Run>,
    }
}

/// Script run get
///
/// Get a specific run
#[endpoint(
    get,
    path = "/channel/{channel_id}/script/{script_id}/run/{run_id}",
    tags = ["script"],
    scopes = [Full],
    permissions = [ScriptInspect],
    response(OK, body = Run, description = "Get script run success"),
)]
pub mod script_run_get {
    use crate::v1::types::{script::Run, ChannelId, RunId, ScriptId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,

        #[path]
        pub run_id: RunId,
    }

    pub struct Response {
        #[json]
        pub run: Run,
    }
}

/// Script run stop
///
/// Stop a running script
#[endpoint(
    post,
    path = "/channel/{channel_id}/script/{script_id}/run/{run_id}/stop",
    tags = ["script"],
    scopes = [Full],
    permissions = [ScriptManage],
    response(NO_CONTENT, description = "Stop script run success"),
)]
pub mod script_run_stop {
    use crate::v1::types::{ChannelId, RunId, ScriptId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,

        #[path]
        pub run_id: RunId,
    }

    pub struct Response {}
}

/// Script run log
///
/// Get logs from a script run
#[endpoint(
    get,
    path = "/channel/{channel_id}/script/{script_id}/run/{run_id}/log",
    tags = ["script"],
    scopes = [Full],
    permissions = [ScriptInspect],
    response(OK, body = Vec<RunLogEntry>, description = "Get script run logs success"),
)]
pub mod script_run_log {
    use crate::v1::types::{script::RunLogEntry, ChannelId, PaginationResponse, RunId, ScriptId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub script_id: ScriptId,

        #[path]
        pub run_id: RunId,
        // FIXME: pagination query for u64
        // #[query]
        // pub query: PaginationQuery<u64>,
    }

    pub struct Response {
        #[json]
        pub logs: PaginationResponse<RunLogEntry>,
    }
}
