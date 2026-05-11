use rquickjs::{class::Trace, Ctx, JsLifetime};

/// logging utilities exposed to scripts
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Logger {}

#[rquickjs::methods]
impl Logger {
    // maybe make this an actual tracer?
    // /// trace level log
    // fn trace(&self, _content: String, _metadata: rquickjs::Object<'_>, _ctx: Ctx<'_>) {
    //     todo!()
    // }

    /// debug level log
    fn debug(&self, _content: String, _metadata: rquickjs::Object<'_>, _ctx: Ctx<'_>) {
        todo!()
    }

    /// info level log
    fn info(&self, _content: String, _metadata: rquickjs::Object<'_>, _ctx: Ctx<'_>) {
        todo!()
    }

    /// warn level log
    fn warn(&self, _content: String, _metadata: rquickjs::Object<'_>, _ctx: Ctx<'_>) {
        todo!()
    }

    /// error level log
    fn error(&self, _content: String, _metadata: rquickjs::Object<'_>, _ctx: Ctx<'_>) {
        todo!()
    }
}
