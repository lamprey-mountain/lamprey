use rquickjs::{class::Trace, JsLifetime};

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
