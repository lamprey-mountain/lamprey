use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue};
use rquickjs::{
    class::{Trace, Tracer},
    Ctx, IntoJs, Iterable, JsLifetime, Result as JsResult,
};
use serde::Deserialize;

/// http request object
#[rquickjs::class]
#[derive(Clone, JsLifetime)]
pub struct Request {
    pub(crate) method: String,
    pub(crate) url: String,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
    // TODO: redirect, statusText, etc
}

/// http response object
#[rquickjs::class]
#[derive(Clone, JsLifetime)]
pub struct Response {
    pub(crate) status: u16,
    pub(crate) url: String,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
    pub(crate) redirected: bool,
    // TODO: statusText, bodyUsed, ok
}

impl<'js> Trace<'js> for Request {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

impl<'js> Trace<'js> for Response {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

/// http headers
#[rquickjs::class]
#[derive(Clone, JsLifetime)]
pub struct Headers {
    pub(crate) headers: HeaderMap,
}

impl<'js> Trace<'js> for Headers {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

/// how redirects should be handled
#[derive(Debug, Default)]
pub enum RequestRedirect {
    #[default]
    Follow,
    Error,
    Manual,
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl Request {
    #[qjs(constructor)]
    fn new(url: String) -> Self {
        Self {
            method: "GET".into(),
            url,
            headers: HeaderMap::new(),
            body: Bytes::new(),
        }
    }

    /// the request url
    #[qjs(get)]
    fn url(&self, _ctx: Ctx<'_>) -> JsResult<String> {
        Ok(self.url.clone())
    }

    /// the http method
    #[qjs(get)]
    fn method(&self, _ctx: Ctx<'_>) -> JsResult<String> {
        Ok(self.method.clone())
    }

    // /// the redirect mode
    // #[qjs(get)]
    // fn redirect(&self, _ctx: Ctx<'_>) -> JsResult<String> {
    //     todo!()
    // }

    /// request headers
    #[qjs(get)]
    fn headers<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Class<'js, Headers>> {
        rquickjs::Class::instance(
            ctx,
            Headers {
                headers: self.headers.clone(),
            },
        )
    }

    // #[qjs(get)]
    // fn clone<'js>(&self, _ctx: Ctx<'js>) -> JsResult<rquickjs::Value<'js>> {
    //     todo!()
    // }

    // TODO: body retrieval: body, blob, arrayBuffer, bytes, json, text
}

#[derive(Deserialize)]
struct ResponseInit {
    status: Option<u16>,
    // TODO: headers
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl Response {
    // FIXME: make init optional
    #[qjs(constructor)]
    fn new(body: rquickjs::Value<'_>, init: rquickjs::Value<'_>) -> JsResult<Self> {
        let init: ResponseInit = rquickjs_serde::from_value_strict(init)
            .map_err(|_| rquickjs::Error::new_from_js("your data", "RequestInit"))?;
        let status = init.status.unwrap_or(200);

        let body_bytes = if body.is_string() {
            Bytes::from(
                body.into_string()
                    .ok_or(rquickjs::Error::new_from_js("your data", "string"))?
                    .to_string()?,
            )
        } else {
            // TODO: support other bodies like uint8 array
            Bytes::new()
        };

        // TODO: support init with headers

        Ok(Self {
            status,
            url: String::new(),
            headers: HeaderMap::new(),
            body: body_bytes,
            redirected: false,
        })
    }

    // #[qjs(static)]
    // fn json() -> JsResult<Self> {}

    // #[qjs(static)]
    // fn error() -> JsResult<Self> {}

    // #[qjs(static)]
    // fn redirect() -> JsResult<Self> {}

    // /// response body
    // fn body<'js>(&self, _ctx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
    //     todo!()
    // }

    /// response headers
    #[qjs(get)]
    fn headers<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Class<'js, Headers>> {
        rquickjs::Class::instance(
            ctx,
            Headers {
                headers: self.headers.clone(),
            },
        )
    }

    /// http status code
    #[qjs(get)]
    fn status(&self, _ctx: Ctx<'_>) -> JsResult<u16> {
        Ok(self.status)
    }

    /// response url
    #[qjs(get)]
    fn url(&self, _ctx: Ctx<'_>) -> JsResult<String> {
        Ok(self.url.clone())
    }

    // fn status_text(&self, _ctx: Ctx<'_>) -> rquickjs::Result<String> {
    //     todo!()
    // }

    // fn body_used(&self, _ctx: Ctx<'_>) -> rquickjs::Result<bool> {
    //     todo!()
    // }

    /// true if response status is a success
    ///
    /// (in the range 200-299)
    #[qjs(get)]
    fn ok(&self) -> JsResult<bool> {
        Ok((200..300).contains(&self.status))
    }

    /// true if response was redirected
    #[qjs(get)]
    fn redirected(&self, _ctx: Ctx<'_>) -> JsResult<bool> {
        Ok(self.redirected)
    }

    // fn text(&self, _ctx: Ctx<'_>) -> rquickjs::Result<()> {
    //     todo!()
    // }

    // fn json(&self, _ctx: Ctx<'_>) -> rquickjs::Result<()> {
    //     todo!()
    // }

    // fn array_buffer(&self, _ctx: Ctx<'_>) -> rquickjs::Result<()> {
    //     todo!()
    // }

    // qjs doesnt have Blob built in
    // fn blob(&self, _ctx: Ctx<'_>) -> rquickjs::Result<()> {
    //     todo!()
    // }

    // qjs doesnt have FormData built in
    // fn form_data(&self, _ctx: Ctx<'_>) -> rquickjs::Result<()> {
    //     todo!()
    // }
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
        Ok(Iterable(entries).into_js(&ctx)?)
    }

    /// iterate over all header keys
    fn keys<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Value<'js>> {
        let keys: Vec<String> = self.headers.keys().map(|k| k.to_string()).collect();
        Ok(Iterable(keys).into_js(&ctx)?)
    }

    /// iterate over all header values
    fn values<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Value<'js>> {
        let values: Vec<String> = self
            .headers
            .values()
            .filter_map(|v| v.to_str().ok())
            .map(String::from)
            .collect();
        Ok(Iterable(values).into_js(&ctx)?)
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

#[rquickjs::module(rename = "lamprey:http")]
pub mod inner {
    pub use super::{Headers, Request, Response};
}
