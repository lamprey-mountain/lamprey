use lamprey_macros::endpoint;

/// Room template create
#[endpoint(
    post,
    path = "/room-template",
    tags = ["room_template"],
    scopes = [Full],
    response(CREATED, body = RoomTemplate, description = "Template created"),
)]
pub mod room_template_create {
    use crate::v1::types::room_template::{RoomTemplate, RoomTemplateCreate};

    pub struct Request {
        #[json]
        pub template: RoomTemplateCreate,
    }

    pub struct Response {
        #[json]
        pub template: RoomTemplate,
    }
}

/// Room template list
#[endpoint(
    get,
    path = "/room-template",
    tags = ["room_template"],
    scopes = [Full],
    response(OK, body = PaginationResponse<RoomTemplate>, description = "Paginate templates"),
)]
pub mod room_template_list {
    use crate::v1::types::room_template::RoomTemplate;
    use crate::v1::types::room_template::RoomTemplateCode;
    use crate::v1::types::{PaginationQuery, PaginationResponse};

    pub struct Request {
        #[query]
        pub pagination: PaginationQuery<RoomTemplateCode>,
    }

    pub struct Response {
        #[json]
        pub templates: PaginationResponse<RoomTemplate>,
    }
}

/// Room template get
#[endpoint(
    get,
    path = "/room-template/{code}",
    tags = ["room_template"],
    scopes = [Full],
    response(OK, body = RoomTemplate, description = "Get template success"),
)]
pub mod room_template_get {
    use crate::v1::types::room_template::{RoomTemplate, RoomTemplateCode};

    pub struct Request {
        #[path]
        pub code: RoomTemplateCode,
    }

    pub struct Response {
        #[json]
        pub template: RoomTemplate,
    }
}

/// Room template edit
#[endpoint(
    patch,
    path = "/room-template/{code}",
    tags = ["room_template"],
    scopes = [Full],
    response(OK, body = RoomTemplate, description = "Edit template success"),
)]
pub mod room_template_edit {
    use crate::v1::types::room_template::{RoomTemplate, RoomTemplateCode, RoomTemplatePatch};

    pub struct Request {
        #[path]
        pub code: RoomTemplateCode,

        #[json]
        pub patch: RoomTemplatePatch,
    }

    pub struct Response {
        #[json]
        pub template: RoomTemplate,
    }
}

/// Room template delete
#[endpoint(
    delete,
    path = "/room-template/{code}",
    tags = ["room_template"],
    scopes = [Full],
    response(NO_CONTENT, description = "Delete template success"),
)]
pub mod room_template_delete {
    use crate::v1::types::room_template::RoomTemplateCode;

    pub struct Request {
        #[path]
        pub code: RoomTemplateCode,
    }

    pub struct Response {}
}

/// Room template sync
#[endpoint(
    post,
    path = "/room-template/{code}/sync",
    tags = ["room_template"],
    scopes = [Full],
    response(OK, body = RoomTemplate, description = "Sync template success"),
)]
pub mod room_template_sync {
    use crate::v1::types::room_template::{RoomTemplate, RoomTemplateCode};

    pub struct Request {
        #[path]
        pub code: RoomTemplateCode,
    }

    pub struct Response {
        #[json]
        pub template: RoomTemplate,
    }
}
