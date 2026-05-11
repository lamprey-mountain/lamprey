use rquickjs::{class::Trace, JsLifetime};

/// global configuration data
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct EnvManager {
    // ...
}

/// an opaue secret that can be used in some apis
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Opaque {
    data: String,
}

#[rquickjs::methods]
impl EnvManager {
    /// lookup a public env value or non opaque secret
    fn get(&self) -> Option<String> {
        todo!()
    }

    /// lookup an opaque env secret
    #[qjs(rename = "get_secret")]
    fn get_secret(&self) -> Option<Opaque> {
        todo!()
    }
}

#[rquickjs::methods]
impl Opaque {
    /// attempt to read this data if available
    fn read(&self) -> String {
        todo!()
    }
}
