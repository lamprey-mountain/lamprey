use lamprey_macros::endpoint;

/// Preferences global put
#[endpoint(
    put,
    path = "/preferences",
    tags = ["preferences"],
    scopes = [Full],
    response(OK, body = PreferencesGlobal, description = "success"),
)]
pub mod preferences_global_put {
    use crate::v1::types::preferences::PreferencesGlobal;

    pub struct Request {
        #[json]
        pub preferences: PreferencesGlobal,
    }

    pub struct Response {
        #[json]
        pub preferences: PreferencesGlobal,
    }
}

/// Preferences room put
#[endpoint(
    put,
    path = "/preferences/room/{room_id}",
    tags = ["preferences"],
    scopes = [Full],
    response(OK, body = PreferencesRoom, description = "success"),
)]
pub mod preferences_room_put {
    use crate::v1::types::preferences::PreferencesRoom;
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub preferences: PreferencesRoom,
    }

    pub struct Response {
        #[json]
        pub preferences: PreferencesRoom,
    }
}

/// Preferences channel put
#[endpoint(
    put,
    path = "/preferences/channel/{channel_id}",
    tags = ["preferences"],
    scopes = [Full],
    response(OK, body = PreferencesChannel, description = "success"),
)]
pub mod preferences_channel_put {
    use crate::v1::types::preferences::PreferencesChannel;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub preferences: PreferencesChannel,
    }

    pub struct Response {
        #[json]
        pub preferences: PreferencesChannel,
    }
}

/// Preferences user put
#[endpoint(
    put,
    path = "/preferences/user/{user_id}",
    tags = ["preferences"],
    scopes = [Full],
    response(OK, body = PreferencesUser, description = "success"),
)]
pub mod preferences_user_put {
    use crate::v1::types::preferences::PreferencesUser;
    use crate::v1::types::UserId;

    pub struct Request {
        #[path]
        pub user_id: UserId,

        #[json]
        pub preferences: PreferencesUser,
    }

    pub struct Response {
        #[json]
        pub preferences: PreferencesUser,
    }
}

/// Preferences global get
#[endpoint(
    get,
    path = "/preferences",
    tags = ["preferences"],
    scopes = [Full],
    response(OK, body = PreferencesGlobal, description = "success"),
)]
pub mod preferences_global_get {
    use crate::v1::types::preferences::PreferencesGlobal;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub preferences: PreferencesGlobal,
    }
}

/// Preferences room get
#[endpoint(
    get,
    path = "/preferences/room/{room_id}",
    tags = ["preferences"],
    scopes = [Full],
    response(OK, body = PreferencesRoom, description = "success"),
)]
pub mod preferences_room_get {
    use crate::v1::types::preferences::PreferencesRoom;
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,
    }

    pub struct Response {
        #[json]
        pub preferences: PreferencesRoom,
    }
}

/// Preferences channel get
#[endpoint(
    get,
    path = "/preferences/channel/{channel_id}",
    tags = ["preferences"],
    scopes = [Full],
    response(OK, body = PreferencesChannel, description = "success"),
)]
pub mod preferences_channel_get {
    use crate::v1::types::preferences::PreferencesChannel;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {
        #[json]
        pub preferences: PreferencesChannel,
    }
}

/// Preferences user get
#[endpoint(
    get,
    path = "/preferences/user/{user_id}",
    tags = ["preferences"],
    scopes = [Full],
    response(OK, body = PreferencesUser, description = "success"),
)]
pub mod preferences_user_get {
    use crate::v1::types::preferences::PreferencesUser;
    use crate::v1::types::UserId;

    pub struct Request {
        #[path]
        pub user_id: UserId,
    }

    pub struct Response {
        #[json]
        pub preferences: PreferencesUser,
    }
}
