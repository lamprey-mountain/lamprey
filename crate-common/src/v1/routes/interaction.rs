use lamprey_macros::endpoint;

/// Interaction create
#[endpoint(
    post,
    path = "/interaction",
    tags = ["interaction"],
    scopes = [Full],
    response(CREATED, body = Interaction, description = "Interaction created successfully"),
)]
pub mod interaction_create {
    use crate::v1::types::interactions::{Interaction, InteractionCreate};

    pub struct Request {
        #[header]
        pub idempotency_key: Option<String>,

        #[json]
        pub create: InteractionCreate,
    }

    pub struct Response {
        #[json]
        pub interaction: Interaction,
    }
}

/// Interaction respond
///
/// Respond to an interaction
#[endpoint(
    post,
    path = "/interaction/{interaction_id}/{token}/callback",
    tags = ["interaction"],
    response(OK, body = InteractionResponse, description = "Interaction response accepted"),
    response(ACCEPTED, description = "Interaction response accepted"),
)]
pub mod interaction_respond {
    use serde::{Deserialize, Serialize};
    use utoipa::{IntoParams, ToSchema};

    use crate::v1::types::interactions::{InteractionResponse, InteractionResponseCreate};
    use crate::v1::types::InteractionId;

    pub struct Request {
        #[path]
        pub interaction_id: InteractionId,

        #[path]
        pub token: String,

        #[query]
        pub query: InteractionResponseQueryParams,

        #[json]
        pub response: InteractionResponseCreate,
    }

    pub struct Response {
        // TODO: implement ?wait=true
        // #[json]
        // pub response: InteractionResponse,
    }

    #[derive(Debug, IntoParams, ToSchema, Serialize, Deserialize)]
    pub struct InteractionResponseQueryParams {
        /// whether to immediately return with 202 accepted or wait to return an `InteractionResponse`
        pub wait: bool,
    }
}
