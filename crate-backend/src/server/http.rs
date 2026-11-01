use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    middleware,
    response::{Html, IntoResponse, Redirect},
    routing::get,
};
use common::v1::types::error::{ApiError, ErrorCode};
use http::{HeaderName, header};
use kerosene_core::config::ListenTransport;
use tower_http::{
    catch_panic::CatchPanicLayer, propagate_header::PropagateHeaderLayer,
    sensitive_headers::SetSensitiveHeadersLayer, trace::TraceLayer,
};
use tracing::warn;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::{
    prelude::*,
    routes::{self, util::script_http::script_http},
    server::http::openapi::ApiDoc,
};

mod frontend;

mod openapi;
mod util;

/// create an axum router for the api
pub fn create_router_api(globals: Globals) -> Router {
    let kerosene_routes = kerosene_rest::Routes::new_api()
        .with_audit_logging(globals.clone())
        .into_axum_openapi()
        .with_state(globals.clone());

    let state = Arc::new(globals.to_server_state());
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api", routes::routes(state.clone()).fallback(api_fallback))
        .with_state(state.clone())
        .merge(kerosene_routes)
        .split_for_parts();

    let latest_version = API_VERSIONS.last().copied().unwrap_or("v1");

    let router = with_versioned_api_docs(router, api)
        .route(
            "/api/docs.json",
            get(|| async { Redirect::permanent("/api/v1/openapi.json") }),
        )
        .route(
            "/api/docs",
            get(|| async { Redirect::permanent("/api/v1/openapi") }),
        )
        .route(
            "/api/latest/openapi.json",
            get(move || async move {
                let path = format!("/api/{latest_version}/openapi.json");
                Redirect::permanent(&path)
            }),
        )
        .route(
            "/api/latest/openapi",
            get(move || async move {
                let path = format!("/api/{latest_version}/openapi");
                Redirect::permanent(&path)
            }),
        );

    let has_ui = state.config.ui.path.is_some() || cfg!(feature = "embed-frontend");

    let router = if has_ui {
        router
            .route(
                "/invite/{code}",
                get(frontend::invite_meta_handler).with_state(state.clone()),
            )
            .fallback_service(
                axum::routing::get(frontend::frontend_handler).with_state(state.clone()),
            )
    } else {
        router.route("/", get(|| async { "it works!" }))
    };

    router
        .layer(middleware::from_fn_with_state(globals.clone(), script_http))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 16))
        .layer(util::cors())
        .layer(SetSensitiveHeadersLayer::new([header::AUTHORIZATION]))
        .layer(TraceLayer::new_for_http())
        // .layer(
        //     TraceLayer::new_for_http().make_span_with(|req: &Request<_>| {
        //         let request_id = req
        //             .headers()
        //             .get("x-request-id")
        //             .and_then(|v| v.to_str().ok())
        //             .unwrap_or("unknown");
        //         tracing::info_span!("http_request", %request_id, method = %req.method(), uri = %req.uri())
        //     }),
        // )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            routes::util::audit_log_middleware,
        ))
    // .layer(SetRequestIdLayer)
}

const API_VERSIONS: &[&str] = &["v1", "v2"];

fn with_versioned_api_docs(router: Router, api: utoipa::openapi::OpenApi) -> Router {
    API_VERSIONS.iter().fold(router, |router, version| {
        let prefix = format!("/api/{version}");
        let mut version_api = api.clone();
        version_api
            .paths
            .paths
            .retain(|path, _| path == &prefix || path.starts_with(&format!("{prefix}/")));

        router
            .route(
                &format!("{prefix}/openapi.json"),
                get(move || async move { Json(version_api) }),
            )
            .route(
                &format!("{prefix}/openapi"),
                get(|| async { Html(include_str!("../html/scalar.html")) }),
            )
    })
}

/// create an axum router for metrics
pub fn create_router_metrics(globals: Globals) -> Router {
    let state = Arc::new(globals.to_server_state());
    Router::new()
        .route("/metrics", get(routes::metrics::get_metrics))
        .with_state(state)
        // NOTE: what layers do i actually need here?
        .layer(util::cors())
        .layer(SetSensitiveHeadersLayer::new([header::AUTHORIZATION]))
        .layer(TraceLayer::new_for_http())
}

/// create an axum router for the media server
pub fn create_router_media(globals: Globals) -> Router {
    let (router, api) = kerosene_rest::Routes::new_media()
        .with_audit_logging(globals.clone())
        .into_axum_openapi()
        .with_state(globals.clone())
        .split_for_parts();

    let router = router
        .route("/api/docs.json", get(|| async { Json(api) }))
        .route(
            "/api/docs",
            get(|| async { Html(include_str!("../html/scalar.html")) }),
        );

    router
        .layer(DefaultBodyLimit::max(1024 * 1024 * 16))
        .layer(util::cors())
        .layer(SetSensitiveHeadersLayer::new([header::AUTHORIZATION]))
        .layer(TraceLayer::new_for_http())
}

/// create an axum router for redex http handlers
pub fn create_router_redexes(globals: Globals) -> Router {
    Router::new()
        .layer(middleware::from_fn_with_state(globals.clone(), script_http))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 16))
        // .layer(util::cors())
        .layer(SetSensitiveHeadersLayer::new([header::AUTHORIZATION]))
        .layer(TraceLayer::new_for_http())
}

pub fn apply_default_middleware(_globals: Globals, router: Router) -> Router {
    // TODO: decide what other layers to apply
    router
        // .layer(DefaultBodyLimit::max(1024 * 1024 * 16))
        // .layer(util::cors())
        // .layer(SetSensitiveHeadersLayer::new([header::AUTHORIZATION]))
        // .layer(TraceLayer::new_for_http())
        // .layer(
        //     TraceLayer::new_for_http().make_span_with(|req: &Request<_>| {
        //         let request_id = req
        //             .headers()
        //             .get("x-request-id")
        //             .and_then(|v| v.to_str().ok())
        //             .unwrap_or("unknown");
        //         tracing::info_span!("http_request", %request_id, method = %req.method(), uri = %req.uri())
        //     }),
        // )
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            "x-trace-id",
        )))
        .layer(CatchPanicLayer::new())
}

async fn api_fallback() -> impl IntoResponse {
    Error::from(ApiError::from_code(ErrorCode::NotFound))
}

/// serve an axum router on a transport
pub async fn serve_transport(transport: ListenTransport, router: Router) -> Result<()> {
    match transport {
        ListenTransport::Tcp { address, port } => {
            let listener = tokio::net::TcpListener::bind((address, port)).await?;
            axum::serve(listener, router).await?;
        }
        ListenTransport::Unix { path } => {
            if let Some(p) = path.parent() {
                tokio::fs::create_dir_all(p).await?;
            }
            if path.exists() {
                warn!("deleting existing socket {}", path.display());
                tokio::fs::remove_file(&path).await?;
            }
            let listener = tokio::net::UnixListener::bind(&path)?;
            let res = axum::serve(listener, router).await;
            let _ = tokio::fs::remove_file(path).await;
            res?;
        }
    }

    Ok(())
}
