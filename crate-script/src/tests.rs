use bytes::Bytes;
use common::{
    v1::types::redex::{EvalInput, EvalStatus},
    v2::types::{EvalId, RedexId, RedexVerId, UserId},
};

use crate::{Engine, ExecutionHandle, Limits, engine::ExecutionEvent};

/// spawn an eval and collect the logs and response it emits until it stops
async fn run(script: &str, input: EvalInput) -> (Vec<String>, Option<http::Response<Bytes>>) {
    let engine = Engine::new(Limits::strict()).unwrap();
    let exec = engine
        .load_js(RedexId::new(), RedexVerId::new(), "test-module", script)
        .await
        .unwrap();
    let mut handle = exec.spawn(input, EvalId::new()).await.unwrap();

    let mut logs = vec![];
    let mut response = None;

    while let Ok(event) = handle.poll().await {
        match &*event {
            ExecutionEvent::Log(entry) => logs.push(entry.content.clone()),
            ExecutionEvent::HttpResponse(res) => response = Some(res.clone()),
            ExecutionEvent::Status(status)
                if matches!(status, EvalStatus::Exited | EvalStatus::Crashed) =>
            {
                break;
            }
            _ => {}
        }
    }

    // NOTE: this panics if the eval crashed
    handle.done().await.unwrap();

    (logs, response)
}

#[tokio::test]
async fn runs_top_level_await() {
    let (logs, _) = run(
        r#"
            await null;
            log.info("before define");
            export const name = "async";
            export function register() {}
        "#,
        EvalInput::Extraction,
    )
    .await;

    assert_eq!(logs, ["before define"]);
}

#[tokio::test]
async fn settles_async_trigger() {
    let (logs, _) = run(
        r#"
            export default { name: "async trigger" };

            export function register(ctl) {
                ctl.onTrigger().id("go").run(async () => {
                    log.info("before await");
                    await Promise.resolve();
                    log.info("after await");
                });
            }
        "#,
        EvalInput::Manual {
            id: "go".to_string(),
            user_id: UserId::new(),
        },
    )
    .await;

    assert_eq!(logs, ["before await", "after await"]);
}

#[tokio::test]
async fn settles_async_http_handler() {
    let (_, response) = run(
        r#"
            import { Response } from "lamprey:http";

            export default { name: "async http" };

            export function register(ctl) {
                ctl.onHttp().id("http").run(async (req) => {
                    await Promise.resolve();
                    return new Response(`got ${req.method}`, { status: 201 });
                });
            }
        "#,
        EvalInput::Http {
            request: http::Request::builder()
                .method("POST")
                .uri("/")
                .body(Bytes::new())
                .unwrap(),
        },
    )
    .await;

    let response = response.expect("handler should have responded");
    assert_eq!(response.status(), 201);
    assert_eq!(response.body().as_ref(), b"got POST");
}
