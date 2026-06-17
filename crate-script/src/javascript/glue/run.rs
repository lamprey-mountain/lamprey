use rquickjs::{Ctx, JsLifetime, Result as JsResult, class::Trace};

/// manages other runs: spawning, sending messages, stopping, etc.
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct RunManager {
    // TODO
}

/// represents a single running script instance
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Run {
    // TODO: script_id, run_id, sender
}

/// the caller's own process, extends Run with receive capability
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct RunSelf {
    // TODO: extends Run with receive queue
}

/// a set of multiple runs (e.g. all runs from one script)
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct RunSet {
    // TODO: store run IDs or references
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl RunManager {
    #[qjs(constructor)]
    fn new() -> JsResult<Self> {
        Err(rquickjs::Error::new_from_js(
            "Request",
            "Can't manually construct this!",
        ))
    }

    /// lookup the caller's own run process
    fn lookup_self<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// lookup any run spawned from a given script
    fn lookup_script<'js>(
        &self,
        _script_id: String,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// lookup a specific run by ID
    fn lookup_run<'js>(
        &self,
        _run_id: String,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// spawn a new run for the given script
    fn spawn<'js>(
        &self,
        _script_id: String,
        _data: rquickjs::Value<'js>,
        _cx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl RunSet {
    /// send a message to all runs in this set
    fn broadcast(&self, _msg: rquickjs::Value<'_>) {
        todo!()
    }

    /// pick an arbitrary run from this set
    fn arbitrary<'js>(&self, _cx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// the script ID this set belongs to
    #[qjs(get)]
    fn script_id(&self, _cx: Ctx<'_>) -> rquickjs::Result<String> {
        todo!()
    }
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl Run {
    /// send a message to this run
    ///
    /// uses erlang-style semantics, may fail
    fn send(&self, _msg: rquickjs::Value<'_>) {
        todo!()
    }

    /// stop this run
    fn stop(&self) {
        todo!()
    }

    /// the script id this run belongs to
    #[qjs(get)]
    fn script_id(&self, _cx: Ctx<'_>) -> rquickjs::Result<String> {
        todo!()
    }

    /// the unique run id
    #[qjs(get)]
    fn id(&self, _cx: Ctx<'_>) -> rquickjs::Result<String> {
        todo!()
    }
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl RunSelf {
    /// receive a message from the queue, optionally with a timeout in ms
    fn receive<'js>(
        &self,
        _timeout: Option<u64>,
        _ctx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }
}

#[rquickjs::module(rename = "lamprey:run")]
pub mod inner {
    pub use super::{Run, RunManager, RunSelf, RunSet};
}
