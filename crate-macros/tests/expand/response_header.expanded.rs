use lamprey_macros::endpoint;
/// Response header test
pub mod response_header {
    use super::*;
    pub struct Request {}
    #[automatically_derived]
    impl ::core::fmt::Debug for Request {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "Request")
        }
    }
    pub struct Response {
        pub test_header: String,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Response {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "Response",
                "test_header",
                &&self.test_header,
            )
        }
    }
    impl crate::v1::routes::ExtractableRoute for Request {
        type Body = ();
        fn extract(
            parts: ::http::request::Parts,
            _body: Self::Body,
        ) -> Result<Self, ::http::Response<::bytes::Bytes>> {
            let path = parts.uri.path();
            let query_str = parts.uri.query().unwrap_or("");
            Ok(Request {})
        }
    }
    pub fn extract_request(
        req: ::http::Request<::bytes::Bytes>,
    ) -> Result<Request, ::http::Response<::bytes::Bytes>> {
        let (parts, body) = req.into_parts();
        let path = parts.uri.path();
        let query_str = parts.uri.query().unwrap_or("");
        Ok(Request {})
    }
    pub fn encode_response(resp: Response) -> ::http::Response<::bytes::Bytes> {
        ::http::Response::builder()
            .status(::http::StatusCode::OK)
            .body(::bytes::Bytes::new())
            .unwrap()
    }
    pub fn encode_request(req: Request) -> ::http::Request<::bytes::Bytes> {
        let mut url = String::from("/test");
        let mut req_builder = ::http::Request::builder()
            .method(::http::Method::GET)
            .uri(&url);
        let body = ::bytes::Bytes::new();
        req_builder.body(body).unwrap()
    }
    pub fn extract_response(
        resp: ::http::Response<::bytes::Bytes>,
    ) -> Result<Response, ::http::Response<::bytes::Bytes>> {
        let status = resp.status();
        if !status.is_success() {
            return Err(resp);
        }
        let (parts, _body) = resp.into_parts();
        Ok(Response {})
    }
    pub fn metadata() -> crate::v1::routes::Endpoint {
        crate::v1::routes::Endpoint {
            operation_id: "response_header",
            summary: "Response header test",
            description: None,
            method: crate::v1::routes::EndpointMethod::Get,
            path: "/test",
            tags: &[],
            tags_full: &[],
            scopes: &[],
            permissions: &[],
            permissions_optional: &[],
            permissions_server: &[],
            permissions_server_optional: &[],
            audit_log_events: &[],
        }
    }
    pub fn update_operation(
        mut op: ::utoipa::openapi::path::OperationBuilder,
    ) -> ::utoipa::openapi::path::OperationBuilder {
        op = op
            .response(
                "200",
                ::utoipa::openapi::ResponseBuilder::new().description("Success").build(),
            );
        op
    }
}
