use axum::{
    Json, Router,
    routing::{MethodRouter, get},
};
use utoipa::openapi::{Components, Info, OpenApiBuilder, PathItem, Tag, extensions::Extensions};

use crate::util::Globals;

pub struct Routes {
    openapi: utoipa::openapi::OpenApi,
    router: Option<Router<Globals>>,
    prefix: String,
    last_path: Option<String>,
}

impl Routes {
    pub fn new() -> Self {
        let info = Info::builder()
            .title("api doccery") // TODO: better name
            .version(env!("CARGO_PKG_VERSION"))
            .description(Some(include_str!("./../../../crate-backend/docs/index.md"))) // TODO: copy docs to somewhere else?
            // .license(env!("CARGO_PKG_LICENSE")) // TODO: parsing license into here
            // .terms_of_service(terms_of_service)
            // .contact(contact)
            .build();

        let openapi = OpenApiBuilder::new()
            .info(info)
            .components(Some(
                // NOTE: im not sure which schemas i need to add to componnts and what will be automatically added
                // see crate-backend/src/serve/mod.rs for what i currently manually add
                // however, i dont know if i'll need to manually add more or less components with this system
                Components::builder()
                    .schema_from::<common::v1::types::ids::UserId>()
                    .schema_from::<common::v1::types::ids::RoomId>()
                    .schema_from::<common::v1::types::ids::ChannelId>()
                    .schema_from::<common::v1::types::ids::MessageId>()
                    .schema_from::<common::v1::types::message::Message>()
                    .build(),
            ))
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

        Self {
            openapi,
            router: Some(Router::new()),
            prefix: String::new(),
            last_path: None,
        }
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

    pub fn nest<F: FnMut(&mut Self)>(&mut self, prefix: &str, mut f: F) {
        // PERF: theres probably some cool way to use std::mem::swap(x, y); instead of cloning
        let old_prefix = self.prefix.clone();
        self.prefix = format!("{}{}", self.prefix, prefix);
        f(self);
        self.prefix = old_prefix;
    }

    /// register a path for the openapi schema
    #[rustfmt::skip]
    pub fn path(&mut self, path: &str, item: PathItem) {
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
    pub fn route(&mut self, path: &str, method_router: MethodRouter<Globals>) {
        let full_path = format!("{}{}", self.prefix, path);
        let r = self.router.take().unwrap();
        self.router = Some(r.route(&full_path, method_router));
        self.last_path = Some(full_path);
    }
}

/// a set of http endpoint handlers
pub trait Handlers {
    fn register(routes: &mut Routes);
}
