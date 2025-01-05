use poem::{listener::TcpListener, Route};
use poem_openapi::{param::Query, payload::PlainText, OpenApi, OpenApiService};

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {}!", name)),
            None => PlainText("hello!".to_string()),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_service = OpenApiService::new(Api, "hello world", "1.0")
        .server("http://localhost:4000/api");
    let ui = api_service.swagger_ui();
    let spec = api_service.spec_endpoint();
    let app = Route::new().nest("/api", api_service)
        .nest("/", ui)
        .nest("/docs.json", spec);
    poem::Server::new(TcpListener::bind("0.0.0.0:4000"))
        .run(app)
        .await?;
    Ok(())
}
