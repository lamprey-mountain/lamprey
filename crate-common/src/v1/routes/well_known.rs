use lamprey_macros::endpoint_new;

// TODO: rename either this route or this module (i have to do well_known::well_known to access the route)
/// Get well known
#[endpoint_new(
    get,
    path = "/.well-known/lamprey-mountain",
    tags = ["federation"],
    response(OK, body = WellKnown),
)]
pub mod well_known {
    use crate::v1::types::federation::WellKnown;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub info: WellKnown,
    }
}
