use bytes::Bytes;

// TODO: use traits for endpoint macro

/// an http endpoint
pub trait Endpoint {
    type Request: Request;
    type Response: Response;

    /// get the metadata for this endpoint
    fn metadata() -> Metadata;
}

// TODO: move struct here
pub use crate::v1::routes::Endpoint as Metadata;

// TODO: move enum here
pub use crate::v1::routes::EndpointMethod as Method;

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

    // TODO: inline ExtractableRoute
    // type Body: DeserializeOwned;

    // /// extract this from http request parts and deserialized Body
    // fn extract_from_parts(
    //     parts: http::request::Parts,
    //     body: Self::Body,
    // ) -> Result<Self, http::Response<Bytes>> {
    //     todo!()
    // }
}
