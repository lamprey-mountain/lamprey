use rquickjs::{class::Trace, Ctx, JsLifetime, Result as JsResult};

// most of this will be implemented in js, this is used internally
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct ApiManagerInner {
    // TODO: reference to API client/connection
}

#[rquickjs::methods]
impl ApiManagerInner {
    #[qjs(constructor)]
    fn new() -> JsResult<Self> {
        Err(rquickjs::Error::new_from_js(
            "Request",
            "Can't manually construct this!",
        ))
    }

    /// do an http fetch
    fn fetch<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}

#[rquickjs::module(rename = "lamprey:api")]
pub mod inner {
    pub use super::ApiManagerInner;
}
