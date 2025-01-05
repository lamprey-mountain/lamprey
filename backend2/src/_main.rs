use anyhow::Result;
use axum::{response::{Html, IntoResponse}, routing::get, Json};
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable as _};

use chat::types;

#[utoipa::path(get, path = "/")]
async fn hello() -> &'static str {
    "hello, world!"
}

#[derive(OpenApi)]
#[openapi(
    paths(hello, openapi),
    components(
        schemas(
            types::Room,
            types::RoomPatch,
            types::User,
            types::Thread,
            types::ThreadPatch,
            types::Message,
            types::Member,
            types::Role,
        )
    ),
)]
struct ApiDoc;

#[utoipa::path(get, path="/api/docs.json")]
async fn openapi() -> impl IntoResponse {
    Json(ApiDoc::openapi())
}

async fn scalar() -> impl IntoResponse {
    Html(Scalar::with_url("/scalar", ApiDoc::openapi()).to_html())
}

#[tokio::main]
async fn main() -> Result<()> {
    let sub = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(sub)?;
    
    let pool = sqlx::PgPool::connect("postgres://chat:ce00eebd05027ca1@localhost:5432/chat").await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    // sqlx::query!()
    // let (router¸ api) = OpenApiRouter::new().routes(routes!());
    let router = axum::Router::new()
        .route("/", get(hello))
        .route("/api/docs.json", get(openapi))
        .route("/api/docs", get(scalar))
    ;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;
    axum::serve(listener, router).await?;
    Ok(())
}
