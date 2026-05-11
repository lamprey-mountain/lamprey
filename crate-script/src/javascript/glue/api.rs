use rquickjs::{class::Trace, Ctx, JsLifetime};

// most of this will be implemented in js, this is used internally
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct ApiManagerInner {
    // TODO: reference to API client/connection
}

#[rquickjs::methods]
impl ApiManagerInner {
    /// do an http fetch
    fn fetch<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}
