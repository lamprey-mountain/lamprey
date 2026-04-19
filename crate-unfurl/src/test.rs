// TODO: test stuff
// eg. snapshot tests

use crate::{error::UnfurlError, script::ScriptRuntime};

#[tokio::test]
async fn test_script() -> Result<(), UnfurlError> {
    let rt = ScriptRuntime::init(None).await?;
    let plug = rt.load("test.js", include_str!("../test.js")).await?;
    dbg!(&plug.name);

    todo!("test script loading and execution");

    Ok(())
}
