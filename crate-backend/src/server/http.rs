use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    middleware,
    response::{Html, IntoResponse},
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

#[cfg(feature = "embed-frontend")]
mod frontend;

mod openapi;
mod util;

// TODO: copy from crate-backend/src/serve/mod.rs

/// create an axum router for the api
pub fn create_router_api(globals: Globals) -> Router {
    let state = Arc::new(globals.to_server_state());
    let (router, mut api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api", routes::routes(state.clone()).fallback(api_fallback))
        .route("/metrics", get(routes::metrics::get_metrics))
        .route("/.well-known/lamprey-mountain", get(routes::well_known))
        .with_state(state.clone())
        .split_for_parts();

    let router = router
        .route("/api/docs.json", get(|| async { Json(api) }))
        .route(
            "/api/docs",
            get(|| async { Html(include_str!("../scalar.html")) }),
        );

    #[cfg(not(feature = "embed-frontend"))]
    let router = router.route("/", get(|| async { "it works!" }));
    #[cfg(feature = "embed-frontend")]
    let router = router
        .route(
            "/invite/{code}",
            get(frontend::invite_meta_handler).with_state(state.clone()),
        )
        .fallback_service(axum::routing::get(frontend::frontend_handler).with_state(state.clone()));

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
        .layer(CatchPanicLayer::new())
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            "x-trace-id",
        )))
    // .layer(SetRequestIdLayer)
}

/// create an axum router for metrics
pub fn create_router_metrics(globals: Globals) -> Router {
    todo!()
}

/// create an axum router for the media server
pub fn create_router_media(_globals: Globals) -> Router {
    todo!()
}

/// create an axum router for redex http handlers
pub fn create_router_redexes(_globals: Globals) -> Router {
    // Router::new().layer(middleware::from_fn_with_state(globals, script_http))
    todo!()
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
