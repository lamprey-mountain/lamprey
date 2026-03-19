use lamprey_macros::endpoint;

/// Ack bulk
#[endpoint(
    post,
    path = "/ack",
    tags = ["ack"],
    scopes = [Full],
    response(NO_CONTENT, description = "ok"),
)]
pub mod ack_bulk {
    use crate::v1::types::ack::AckBulk;

    pub struct Request {
        #[json]
        pub ack: AckBulk,
    }

    pub struct Response;
}
