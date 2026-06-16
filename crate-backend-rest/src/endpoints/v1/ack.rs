use crate::prelude::*;

use axum::{body::Body, extract::State, response::IntoResponse, routing::MethodFilter};
use common::{
    util::routes::{Endpoint, Response},
    v1::routes,
};
use utoipa::openapi::{
    Content, HttpMethod, PathItem, PathsBuilder, Response, Responses, path::Operation,
    request_body::RequestBody,
};

pub struct Endpoints {
    // ...
}

// impl Endpoints {
//     pub fn new(globals: Globals) -> Self {
//         todo!()
//     }
// }

#[handlers]
impl Endpoints {
    #[endpoint(routes::ack_bulk)]
    pub async fn bulk(
        &self,
        req: Req<routes::ack_bulk_new::Endpoint>,
    ) -> Result<routes::ack_bulk_new::Response> {
        // req.auth.ensure_scopes(&[Scope::Full])?;
        // TODO

        Ok(routes::ack_bulk_new::Response {})
    }
}

// the above should roughly expand to this:

impl Handlers for Endpoints {
    fn register(r: &mut Routes) {
        async fn handler(req: Req<routes::ack_bulk_new::Endpoint>) -> Result<impl IntoResponse> {
            let e: Endpoints = todo!("somehow get access to Endpoints here???");
            e.bulk(req).await.map(|r| r.encode().map(Body::from))
        }

        let endpoint = routes::ack_bulk_new::Endpoint::metadata();
        r.router = r
            .router
            .route("/ack", axum::routing::on(MethodFilter::POST, handler));
        let p = PathItem::new(
            HttpMethod::Get,
            Operation::builder()
                .summary(Some(endpoint.summary))
                .description(endpoint.description)
                .tags(Some(endpoint.tags_full.iter().map(|s| s.to_string())))
                .operation_id(Some(endpoint.operation_id))
                .request_body(Some(
                    RequestBody::builder()
                        // .content(content_type, Content::new(schema))
                        // .required(required)
                        // .description(description)
                        // .extensions(extensions)
                        .build(),
                ))
                // .response(
                //     code, // http status code
                //     Response::builder()
                //         .description(description)
                //         .content(content_type, content)
                //         .header(name, header)
                //         .extensions(extensions)
                //         .link(name, link)
                //         .build(),
                // )
                .build(),
        );
        r.path(p);
    }
}
