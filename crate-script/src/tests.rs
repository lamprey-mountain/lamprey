use common::{
    v1::types::redex::EvalInput,
    v2::types::{EvalId, RedexId, RedexVerId},
};

use crate::{Engine, Limits};

#[tokio::test] // NOTE: idk if i should have tokio in this crate or not, but i do think i definitely need it for testing at least?
async fn test_foo() {
    let source = r#"
            log.info("yo")
        "#;

    let engine = Engine::new(Limits::strict()).unwrap();
    let exec = engine
        .load_js(RedexId::new(), RedexVerId::new(), "my-cool-module", source)
        .await
        .unwrap();
    let mut handle = exec
        .spawn(EvalInput::Extraction, EvalId::new())
        .await
        .unwrap();
    dbg!(handle.done().await.unwrap());
    todo!("finish writing tests")
}

// TODO: write tests
