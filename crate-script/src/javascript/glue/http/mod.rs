pub mod form_data;
pub mod headers;
pub mod request;
pub mod response;

pub use form_data::FormData;
pub use headers::Headers;
pub use request::Request;
pub use response::Response;

// /// how redirects should be handled
// #[derive(Debug, Default)]
// pub enum RequestRedirect {
//     #[default]
//     Follow,
//     Error,
//     Manual,
// }

#[rquickjs::module(rename = "lamprey:http")]
pub mod inner {
    pub use super::{FormData, Headers, Request, Response};
}

// TODO: .blob() and .formData() extractors for Request/Response? (quickjs doesnt have these built in) (necessary for compatibility but low priority)
