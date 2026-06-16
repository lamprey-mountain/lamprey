use bytes::Bytes;

/// an http endpoint
pub trait Endpoint {
    type Request: Request;
    type Response: Response;

    /// get the metadata for this endpoint
    fn metadata() -> Metadata;
}

// TODO: move struct here
pub use crate::v1::routes::Endpoint as Metadata;

/// HTTP method for an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
}

pub trait Request: Sized {
    /// encode this into an http request
    fn encode(self) -> http::Request<Bytes>;

    /// extract this from an http request
    ///
    /// on failure, returns the original http request
    fn extract(req: http::Request<Bytes>) -> Result<Self, http::Request<Bytes>>;
}

pub trait Response: Sized {
    /// encode this into an http response
    fn encode(self) -> http::Response<Bytes>;

    /// extract this from an http response
    ///
    /// on failure, returns the original http response
    fn extract(req: http::Response<Bytes>) -> Result<Self, http::Response<Bytes>>;
}
