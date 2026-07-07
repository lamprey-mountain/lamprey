use http::{HeaderMap, HeaderName, HeaderValue};
use rquickjs::{
    Ctx, IntoJs, Iterable, JsLifetime, Result as JsResult,
    class::{Trace, Tracer},
};

/// http headers
#[rquickjs::class]
#[derive(Clone, JsLifetime)]
pub struct Headers {
    pub(crate) headers: HeaderMap,
}

impl<'js> Trace<'js> for Headers {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl Headers {
    #[qjs(constructor)]
    fn new() -> Self {
        Self {
            headers: HeaderMap::new(),
        }
    }

    /// append a value to a header
    fn append(&mut self, _ctx: Ctx<'_>, name: String, value: String) -> JsResult<()> {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| rquickjs::Error::new_from_js_message("string", "name", e.to_string()))?;
        let value = HeaderValue::from_str(&value)
            .map_err(|e| rquickjs::Error::new_from_js_message("string", "value", e.to_string()))?;
        self.headers.append(name, value);
        Ok(())
    }

    /// remove a header
    fn delete(&mut self, _ctx: Ctx<'_>, name: String) -> JsResult<()> {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| rquickjs::Error::new_from_js_message("string", "name", e.to_string()))?;
        self.headers.remove(name);
        Ok(())
    }

    /// get a header value
    fn get(&self, _ctx: Ctx<'_>, name: String) -> Option<String> {
        let name = HeaderName::from_bytes(name.as_bytes()).ok()?;
        self.headers
            .get(&name)
            .and_then(|v| v.to_str().ok().map(|s| s.to_string()))
    }

    /// check if a header exists
    fn has(&self, _ctx: Ctx<'_>, name: String) -> bool {
        let name = match HeaderName::from_bytes(name.as_bytes()) {
            Ok(n) => n,
            Err(_) => return false,
        };
        self.headers.contains_key(&name)
    }

    /// set a header value
    fn set(&mut self, _ctx: Ctx<'_>, name: String, value: String) -> JsResult<()> {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| rquickjs::Error::new_from_js_message("string", "name", e.to_string()))?;
        let value = HeaderValue::from_str(&value)
            .map_err(|e| rquickjs::Error::new_from_js_message("string", "value", e.to_string()))?;
        self.headers.insert(name, value);
        Ok(())
    }

    /// iterate over all header entries
    fn entries<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Value<'js>> {
        let entries: Vec<Vec<String>> = self
            .headers
            .iter()
            .map(|(name, value)| vec![name.to_string(), value.to_str().unwrap_or("").to_string()])
            .collect();
        Ok(Iterable::from(entries).into_js(&ctx)?)
    }

    /// iterate over all header keys
    fn keys<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Value<'js>> {
        let keys: Vec<String> = self.headers.keys().map(|k| k.to_string()).collect();
        Ok(Iterable::from(keys).into_js(&ctx)?)
    }

    /// iterate over all header values
    fn values<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Value<'js>> {
        let values: Vec<String> = self
            .headers
            .values()
            .filter_map(|v| v.to_str().ok())
            .map(String::from)
            .collect();
        Ok(Iterable::from(values).into_js(&ctx)?)
    }

    /// call a function for each header
    fn for_each(&self, _ctx: Ctx<'_>, callback: rquickjs::Function<'_>) -> JsResult<()> {
        for (name, value) in self.headers.iter() {
            let name_str = name.to_string();
            let value_str = value.to_str().unwrap_or("").to_string();
            callback.call::<_, ()>((value_str, name_str, self.clone()))?;
        }
        Ok(())
    }

    /// get the Set-Cookie header(s)
    fn get_set_cookie(&self) -> Vec<String> {
        let name = HeaderName::from_static("set-cookie");
        self.headers
            .get_all(&name)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(String::from)
            .collect()
    }
}
