use serde::{de::DeserializeOwned, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::v1::types::{Channel, ChannelCreate};

pub enum Method {
    Head,
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// a single http endpoint
pub trait Endpoint {
    const METHOD: Method;
    const PATH: &'static str;
    const SUMMARY: &'static str;
    const DOC: &'static str;

    type Request: Serialize + DeserializeOwned + ToSchema;

    type Response: Serialize + DeserializeOwned + ToSchema;

    // how can i make this optional?
    // type RequestQuery: Serialize + DeserializeOwned + ToSchema + IntoParams;
    // #[derive(Serialize, Deserialize, ToSchema, IntoParams)]
    // pub struct Nothing;
    // maybe just copy ruma instead

    // TODO: list route requirements
}

// pub trait PathBuilder {}

// /// a request the server is sending
// pub trait IncomingRequest {}

// /// a request the client is receiving
// pub trait IncomingResponse {}

// /// a request the client is sending
// pub trait OutgoingRequest {}

// /// a response the server is sending
// pub trait OutgoingResponse {}

// route restrictions:
//
// - requires valid auth (every route except POST /session?)
// - requires valid user (most routes)
// - requires unsuspended (most write routes)
// - requires sudo mode
// - room may require sudo mode
// - room may require mfa
// - ratelimit bucket
// - requires room permission
// - requires server permission (eg. admin only)
// - requires oauth scope

pub struct ChannelCreateRoom;

impl Endpoint for ChannelCreateRoom {
    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/room/{room_id}/channel";
    const SUMMARY: &'static str = "Room channel create";
    const DOC: &'static str = "Create a channel in a room";

    type Request = ChannelCreate;
    type Response = Channel;
}

/// Room channel create
///
/// Create a channel in a room
// #[endpoint(
//     method(POST),
// )]
#[cfg(any())]
mod channel_create_room {
    metadata! {
        method: POST,
        path: "/room/{room_id}/channel",
    }

    // a potential downside with this method is needing to redefine all fields again every time
    // if i have multiple similar routes, this could become very unwieldy
    #[request]
    pub struct Request {
        #[api(path)]
        pub room_id: RoomId,

        #[api(query)]
        // #[api(query("param"))]
        pub query_param: String,

        #[api(header("Idempotenty-Key"))]
        pub idempotenty_key: String,

        #[serde(default)]
        #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 64))]
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
        pub name: String,

        #[cfg_attr(
            feature = "utoipa",
            schema(required = false, max_length = 1, min_length = 2048)
        )]
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
        pub description: Option<String>,
    }

    #[response]
    pub struct Response {
        #[ruma_api(header = CONTENT_TYPE)]
        pub content_type: String,

        #[ruma_api(raw_body)]
        pub file: Vec<u8>,
    }
}

#[cfg(feature = "utoipa")]
mod utoipa_impl {
    use utoipa::openapi::path::{HttpMethod, Operation};

    use crate::v1::routes::{Endpoint, Method};

    impl Into<HttpMethod> for Method {
        fn into(self) -> HttpMethod {
            match self {
                Method::Head => HttpMethod::Head,
                Method::Get => HttpMethod::Get,
                Method::Post => HttpMethod::Post,
                Method::Put => HttpMethod::Put,
                Method::Patch => HttpMethod::Patch,
                Method::Delete => HttpMethod::Delete,
            }
        }
    }

    #[cfg(any())]
    impl<E: Endpoint> utoipa::Path for E {
        fn path() -> String {
            E::PATH.into()
        }

        fn methods() -> Vec<HttpMethod> {
            vec![E::METHOD.into()]
        }

        fn operation() -> Operation {
            todo!()
        }
    }
}
