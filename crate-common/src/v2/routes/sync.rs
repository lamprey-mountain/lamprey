use lamprey_macros::endpoint;

// TODO: this route definition seems kind of janky, unsure how to improve it
/// Sync init
///
/// multipurpose endpoint that can be used to
///
/// - connect sync via websocket
/// - connect sync via webtransport
/// - get sync limits
#[endpoint(
    get,
    path = "/sync",
    tags = ["sync"],
    scopes = [Full],
    response(101, description = "Init websocket"),
    response(OK, body = SyncLimits, description = "Get sync limits"),
)]
pub mod sync_init {
    use crate::v2::types::sync::{WebsocketSyncParams, shard::SyncLimits};

    pub struct Request {
        // TODO: make optional
        #[query]
        pub query: WebsocketSyncParams,
    }

    pub struct Response {
        // TODO: make optional
        #[json]
        pub limits: SyncLimits,
    }
}

/// Sync create
#[endpoint(
    post,
    path = "/sync",
    tags = ["sync"],
    scopes = [Full],
    response(CREATED, body = Syncer, description = "success"),
)]
pub mod sync_create {
    use crate::v2::types::sync::shard::{Syncer, SyncerCreate};

    pub struct Request {
        #[json]
        pub body: SyncerCreate,
    }

    pub struct Response {
        #[json]
        pub syncer: Syncer,
    }
}

/// Sync get
#[endpoint(
    get,
    path = "/sync/{sync_id}",
    tags = ["sync"],
    scopes = [Full],
    response(OK, body = Syncer, description = "success"),
)]
pub mod sync_get {
    use crate::v2::types::SyncId;
    use crate::v2::types::sync::shard::Syncer;

    pub struct Request {
        #[path]
        pub sync_id: SyncId,
    }
    pub struct Response {
        #[json]
        pub syncer: Syncer,
    }
}

/// Sync delete
#[endpoint(
    delete,
    path = "/sync/{sync_id}",
    tags = ["sync"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod sync_delete {
    use crate::v2::types::SyncId;

    pub struct Request {
        #[path]
        pub sync_id: SyncId,
    }
    pub struct Response {}
}

/// Shard create
#[endpoint(
    post,
    path = "/sync/{sync_id}/shard",
    tags = ["sync"],
    scopes = [Full],
    response(CREATED, body = Shard, description = "success"),
)]
pub mod shard_create {
    use crate::v2::types::SyncId;
    use crate::v2::types::sync::shard::{Shard, ShardCreate};

    pub struct Request {
        #[path]
        pub sync_id: SyncId,

        #[json]
        pub body: ShardCreate,
    }

    pub struct Response {
        #[json]
        pub shard: Shard,
    }
}

/// Shard delete
#[endpoint(
    delete,
    path = "/sync/{sync_id}/shard/{shard_id}",
    tags = ["sync"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod shard_delete {
    use crate::v2::types::{ShardId, SyncId};

    pub struct Request {
        #[path]
        pub sync_id: SyncId,

        #[path]
        pub shard_id: ShardId,
    }

    pub struct Response {}
}
