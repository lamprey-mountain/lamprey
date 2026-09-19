use std::mem;

use axum::{
    Json, Router,
    routing::{MethodRouter, get},
};
use common::util::registry::Registry;
use utoipa::{
    ToSchema,
    openapi::{
        Components, ComponentsBuilder, Info, OpenApi, OpenApiBuilder, PathItem, Tag,
        extensions::Extensions,
    },
};
use utoipa_axum::router::OpenApiRouter;

use crate::prelude::*;

pub struct Routes {
    openapi: OpenApi,
    router: Option<Router<Globals>>,
    prefix: String,
    last_path: Option<String>,
}

#[derive(Default)]
struct UtoipaRegistry(ComponentsBuilder);

impl Registry for UtoipaRegistry {
    fn register<T: ToSchema>(&mut self) {
        self.0 = mem::take(&mut self.0).schema_from::<T>();
    }
}

impl UtoipaRegistry {
    #[inline]
    pub fn build(self) -> Components {
        self.0.build()
    }
}

impl Routes {
    /// create a new Routes for all api routes
    pub fn new_api() -> Self {
        let info = Info::builder()
            .title("Lamprey Mountain API")
            .version(env!("CARGO_PKG_VERSION"))
            .description(Some(include_str!("./../../../crate-backend/docs/index.md"))) // TODO: copy docs to somewhere else?
            // .license(env!("CARGO_PKG_LICENSE")) // TODO: parsing license into here
            // .terms_of_service(terms_of_service)
            // .contact(contact)
            .build();

        // collect all types/models/openapi schemas
        let mut registry = UtoipaRegistry::default();
        common::v1::types::register(&mut registry);

        let openapi = OpenApiBuilder::new()
            .info(info)
            .components(Some(registry.build()))
            // copy crate-backend/src/serve/utoipa_utils.rs here
            .tags(Some([Tag::builder()
                .name("auth")
                .description(Some("authentication and session management"))
                .extensions(Some(
                    Extensions::builder()
                        .add("x-displayName", "auth".replace("_", " "))
                        .build(),
                ))
                .build()]))
            .build();

        // TODO: copy crate-backend/src/serve/utoipa_utils.rs BadgeModifier

        // TODO: populate servers?
        // .servers(Some([utoipa::openapi::Server::builder()
        //     .url(url)
        //     .description(description)
        //     .parameter(name, variable)
        //     .build()]));

        // // TODO: copy crate-backend/src/serve/utoipa_utils.rs NestedTags
        // openapi
        //     .extensions
        //     .get_or_insert_default()
        //     .merge(Extensions::builder().add("x-tagGroups", todo!()).build());

        let mut me = Self {
            openapi,
            router: Some(Router::new()),
            prefix: String::new(),
            last_path: None,
        };

        crate::endpoints::api::register(&mut me);

        me
    }

    /// create a new Routes for all media/cdn routes
    pub fn new_media() -> Self {
        let info = Info::builder()
            .title("Lamprey Mountain CDN")
            .version(env!("CARGO_PKG_VERSION"))
            .description(Some("documentation for the cdn")) // TODO: write more docs
            .build();

        // TODO: collect all types/models/openapi schemas for the cdn
        // let mut registry = UtoipaRegistry::default();
        // common::v1::types::register(r);

        let openapi = OpenApiBuilder::new().info(info).build();

        let mut me = Self {
            openapi,
            router: Some(Router::new()),
            prefix: String::new(),
            last_path: None,
        };

        crate::endpoints::media::register(&mut me);

        me
    }

    /// get a reference the openapi schema
    pub fn openapi(&self) -> &OpenApi {
        &self.openapi
    }

    /// get a reference the axum router
    pub fn router(&self) -> &Router<Globals> {
        self.router.as_ref().unwrap()
    }

    /// convert this into an axum router
    pub fn into_axum(self) -> Router<Globals> {
        self.router
            .unwrap()
            .route("/api/docs.json", get(|| async { Json(self.openapi) }))
        // // TODO(?): maybe i could have an authenticated openapi schema endpoint
        // // only return endpoints the current session can use
        // .route(
        //     "/api/docs-authenticated.json",
        //     get(|req: super::Req<_>| async {
        //         // self.openapi.clone();
        //         req.auth.scopes();
        //         let openapi_filtered = todo!();
        //         Json(openapi_filtered)
        //     }),
        // )
    }

    /// convert this into an OpenApiRouter router
    pub fn into_axum_openapi(self) -> OpenApiRouter<Globals> {
        let router: OpenApiRouter<Globals> = self.router.unwrap().into();
        let schema: OpenApiRouter<Globals> = OpenApiRouter::with_openapi(self.openapi);
        router.merge(schema)
    }

    pub(crate) fn nest<F: FnMut(&mut Self)>(&mut self, prefix: &str, mut f: F) {
        // PERF: theres probably some cool way to use std::mem::swap(x, y); instead of cloning
        let old_prefix = self.prefix.clone();
        self.prefix = format!("{}{}", self.prefix, prefix);
        f(self);
        self.prefix = old_prefix;
    }

    /// register a path for the openapi schema
    #[rustfmt::skip]
    pub(crate) fn path(&mut self, path: &str, item: PathItem) {
        use std::collections::btree_map::Entry;
        match self.openapi.paths.paths.entry(path.to_string()) {
            Entry::Vacant(v) => {
                v.insert(item);
            },
            Entry::Occupied(mut p) => {
                let p  = p.get_mut();
                if let Some(op) = item.get { p.get = Some(op); }
                if let Some(op) = item.post { p.post = Some(op); }
                if let Some(op) = item.put { p.put = Some(op); }
                if let Some(op) = item.delete { p.delete = Some(op); }
                if let Some(op) = item.options { p.options = Some(op); }
                if let Some(op) = item.head { p.head = Some(op); }
                if let Some(op) = item.patch { p.patch = Some(op); }
                if let Some(op) = item.trace { p.trace = Some(op); }
            },
        }
    }

    /// register a new axum route
    pub(crate) fn route(&mut self, path: &str, method_router: MethodRouter<Globals>) {
        let full_path = format!("{}{}", self.prefix, path);
        let r = self.router.take().unwrap();
        self.router = Some(r.route(&full_path, method_router));
        self.last_path = Some(full_path);
    }
}
