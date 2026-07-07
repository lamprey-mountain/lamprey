use lamprey_macros::endpoint;

/// Response header test
#[endpoint(
    get,
    path = "/test",
)]
pub mod response_header {
    pub struct Request {}
    pub struct Response {
        #[header(rename = "X-Test-Header")]
        pub test_header: String,
    }
}
