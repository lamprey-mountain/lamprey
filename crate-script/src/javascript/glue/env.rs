use rquickjs::{JsLifetime, Result as JsResult, class::Trace};

/// global configuration data
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct EnvManager {
    // ...
}

/// an opaque secret that can be used in some apis
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Opaque {
    data: String,
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl EnvManager {
    #[qjs(constructor)]
    fn new() -> JsResult<Self> {
        Err(rquickjs::Error::new_from_js(
            "Request",
            "Can't manually construct this!",
        ))
    }

    /// lookup a public env value or non opaque secret
    fn get(&self) -> Option<String> {
        todo!()
    }

    /// lookup an opaque env secret
    fn get_secret(&self) -> Option<Opaque> {
        todo!()
    }
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl Opaque {
    /// attempt to read this data if available
    fn read(&self) -> String {
        todo!()
    }
}

#[rquickjs::module(rename = "lamprey:env")]
pub mod inner {
    pub use super::{EnvManager, Opaque};
}
