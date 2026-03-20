use lamprey_macros::endpoint;

/// Automod rule list
#[endpoint(
    get,
    path = "/room/{room_id}/automod/rule",
    tags = ["automod"],
    scopes = [Full],
    permissions = [RoomEdit],
    response(OK, body = Vec<AutomodRule>, description = "List automod rules success"),
)]
pub mod automod_rule_list {
    use crate::v1::types::automod::AutomodRule;
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,
    }

    pub struct Response {
        #[json]
        pub rules: Vec<AutomodRule>,
    }
}

/// Automod rule create
#[endpoint(
    post,
    path = "/room/{room_id}/automod/rule",
    tags = ["automod"],
    scopes = [Full],
    permissions = [RoomEdit],
    response(CREATED, body = AutomodRule, description = "Create automod rule success"),
)]
pub mod automod_rule_create {
    use crate::v1::types::automod::{AutomodRule, AutomodRuleCreate};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub rule: AutomodRuleCreate,
    }

    pub struct Response {
        #[json]
        pub rule: AutomodRule,
    }
}

/// Automod rule get
#[endpoint(
    get,
    path = "/room/{room_id}/automod/rule/{rule_id}",
    tags = ["automod"],
    scopes = [Full],
    permissions = [RoomEdit],
    response(OK, body = AutomodRule, description = "Get automod rule success"),
)]
pub mod automod_rule_get {
    use crate::v1::types::automod::AutomodRule;
    use crate::v1::types::{AutomodRuleId, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub rule_id: AutomodRuleId,
    }

    pub struct Response {
        #[json]
        pub rule: AutomodRule,
    }
}

/// Automod rule update
#[endpoint(
    patch,
    path = "/room/{room_id}/automod/rule/{rule_id}",
    tags = ["automod"],
    scopes = [Full],
    permissions = [RoomEdit],
    response(OK, body = AutomodRule, description = "Update automod rule success"),
)]
pub mod automod_rule_update {
    use crate::v1::types::automod::{AutomodRule, AutomodRuleUpdate};
    use crate::v1::types::{AutomodRuleId, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub rule_id: AutomodRuleId,

        #[json]
        pub rule: AutomodRuleUpdate,
    }

    pub struct Response {
        #[json]
        pub rule: AutomodRule,
    }
}

/// Automod rule delete
#[endpoint(
    delete,
    path = "/room/{room_id}/automod/rule/{rule_id}",
    tags = ["automod"],
    scopes = [Full],
    permissions = [RoomEdit],
    response(NO_CONTENT, description = "Delete automod rule success"),
)]
pub mod automod_rule_delete {
    use crate::v1::types::{AutomodRuleId, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub rule_id: AutomodRuleId,
    }

    pub struct Response {}
}

/// Automod rule test
#[endpoint(
    post,
    path = "/room/{room_id}/automod/rule/test",
    tags = ["automod"],
    scopes = [Full],
    permissions = [RoomEdit],
    response(OK, body = AutomodRuleTest, description = "Test automod rule success"),
)]
pub mod automod_rule_test {
    use crate::v1::types::automod::{AutomodRuleTest, AutomodRuleTestRequest};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub test: AutomodRuleTestRequest,
    }

    pub struct Response {
        #[json]
        pub test: AutomodRuleTest,
    }
}
