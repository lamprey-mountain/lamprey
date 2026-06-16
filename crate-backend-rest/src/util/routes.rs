use axum::{Json, Router, routing::get};
use utoipa::openapi::{Info, OpenApiBuilder, PathItem};

use crate::util::Globals;

pub struct Routes {
    pub(crate) openapi_builder: OpenApiBuilder,
    pub(crate) router: Router<Globals>,
}

impl Routes {
    fn new() -> Self {
        let info = Info::builder()
            .title("api doccery") // TODO: better name
            .version(env!("CARGO_PKG_VERSION"))
            .description(Some(include_str!("./../../../crate-backend/docs/index.md"))) // TODO: copy docs to somewhere else?
            // .license(env!("CARGO_PKG_LICENSE"))
            // .terms_of_service(terms_of_service)
            // .contact(contact)
            .build();
        let b = OpenApiBuilder::new().info(info);
        // .tags(Some([Tag::builder()
        //     .name(name)
        //     .description(description)
        //     .external_docs(external_docs)
        //     .extensions(extensions)
        //     .build()]));
        // .servers(Some([utoipa::openapi::Server::builder()
        //     .url(url)
        //     .description(description)
        //     .parameter(name, variable)
        //     .build()]));

        Self {
            openapi_builder: b,
            router: Router::new(),
        }
    }

    /// convert this into an axum router
    pub fn into_axum(self) -> Router<Globals> {
        let openapi = self.openapi_builder.build();
        self.router
            .route("/api/docs.json", get(|| async { Json(openapi) }))
    }

    pub fn nest<F: FnMut(&mut Self)>(&mut self, prefix: &str, f: F) {
        todo!()
    }

    /// register a path
    pub fn path(&mut self, item: PathItem) {
        todo!()
    }
}

/// a set of http endpoint handlers
pub trait Handlers {
    fn register(routes: &mut Routes);
}
