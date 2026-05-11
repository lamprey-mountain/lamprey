use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue};
use rquickjs::{
    class::{Trace, Tracer},
    prelude::Opt,
    Ctx, FromJs, IntoJs, Iterable, JsLifetime, Result as JsResult,
};

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

// /// how redirects should be handled
// #[derive(Debug, Default)]
// pub enum RequestRedirect {
//     #[default]
//     Follow,
//     Error,
//     Manual,
// }

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

    /// returns the body as text
    fn text<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Promise<'js>> {
        let (promise, resolve, _reject) = ctx.promise()?;
        let text = String::from_utf8_lossy(&self.body).to_string();
        resolve.call::<_, ()>((text,))?;
        Ok(promise)
    }

    /// returns the body parsed as JSON
    fn json<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Promise<'js>> {
        let (promise, resolve, reject) = ctx.promise()?;

        match serde_json::from_slice::<serde_json::Value>(&self.body) {
            Ok(value) => {
                match rquickjs_serde::to_value(ctx.clone(), value) {
                    Ok(js_val) => resolve.call::<_, ()>((js_val,)),
                    Err(e) => reject.call::<_, ()>((rquickjs::Exception::from_message(
                        ctx,
                        &format!("Failed to serialize JSON: {}", e),
                    )
                    .map_err(|_| rquickjs::Error::Exception)?,)),
                }?;
            }
            Err(e) => {
                let error_msg = format!("Failed to parse JSON: {}", e);
                let js_error = rquickjs::Exception::from_message(ctx, &error_msg)?;
                reject.call::<_, ()>((js_error,))?;
            }
        }

        Ok(promise)
    }

    /// returns the body as an ArrayBuffer
    fn array_buffer<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Promise<'js>> {
        let (promise, resolve, _reject) = ctx.promise()?;
        let ab = rquickjs::ArrayBuffer::new(ctx, self.body.as_ref())?;
        resolve.call::<_, ()>((ab,))?;
        Ok(promise)
    }
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl Response {
    #[qjs(constructor)]
    fn new<'js>(
        body: rquickjs::Value<'js>,
        init: Opt<rquickjs::Value<'js>>,
        ctx: Ctx<'js>,
    ) -> JsResult<Self> {
        let mut status = 200;
        let mut headers_js: Option<rquickjs::Value<'js>> = None;

        // extract values from init object directly
        if let Some(init_val) = init.0 {
            if !init_val.is_undefined() && !init_val.is_null() {
                let obj = init_val.into_object().ok_or_else(|| {
                    rquickjs::Error::new_from_js("ResponseInit", "expected object")
                })?;

                if let Some(s) = obj.get::<_, Option<u16>>("status")? {
                    status = s;
                }

                headers_js = obj.get::<_, Option<rquickjs::Value<'js>>>("headers")?;
            }
        }

        // parse headers
        let mut headers = HeaderMap::new();
        if let Some(h_val) = headers_js {
            if let Ok(headers_class) = rquickjs::Class::<Headers>::from_js(&ctx, h_val.clone()) {
                // headers class
                headers = headers_class.borrow().headers.clone();
            } else if let Some(obj) = h_val.as_object() {
                // plain js object
                for pair in obj.props::<String, String>() {
                    let (key, value) = pair?;
                    let name = HeaderName::from_bytes(key.as_bytes()).map_err(|e| {
                        rquickjs::Error::new_from_js_message(
                            "header",
                            "invalid name",
                            e.to_string(),
                        )
                    })?;
                    let val = HeaderValue::from_str(&value).map_err(|e| {
                        rquickjs::Error::new_from_js_message(
                            "header",
                            "invalid value",
                            e.to_string(),
                        )
                    })?;
                    headers.append(name, val);
                }
            } else if !h_val.is_undefined() && !h_val.is_null() {
                // ignore undefined/null, reject everything else
                return Err(rquickjs::Error::new_from_js(
                    "Headers",
                    "expected object or Headers instance",
                ));
            }
        }

        let body_bytes = if body.is_string() {
            Bytes::from(body.into_string().unwrap().to_string()?)
        } else {
            // TODO: support other bodies like uint8 array
            Bytes::new()
        };

        Ok(Self {
            status,
            headers,
            body: body_bytes,
            url: String::new(),
            redirected: false,
        })
    }

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

    /// returns the body as text
    fn text<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Promise<'js>> {
        let (promise, resolve, _reject) = ctx.promise()?;
        let text = String::from_utf8_lossy(&self.body).to_string();
        resolve.call::<_, ()>((text,))?;
        Ok(promise)
    }

    /// returns the body parsed as JSON
    fn json<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Promise<'js>> {
        let (promise, resolve, reject) = ctx.promise()?;

        match serde_json::from_slice::<serde_json::Value>(&self.body) {
            Ok(value) => {
                match rquickjs_serde::to_value(ctx.clone(), value) {
                    Ok(js_val) => resolve.call::<_, ()>((js_val,)),
                    Err(e) => reject.call::<_, ()>((rquickjs::Exception::from_message(
                        ctx,
                        &format!("Failed to serialize JSON: {}", e),
                    )
                    .map_err(|_| rquickjs::Error::Exception)?,)),
                }?;
            }
            Err(e) => {
                let error_msg = format!("Failed to parse JSON: {}", e);
                let js_error = rquickjs::Exception::from_message(ctx, &error_msg)?;
                reject.call::<_, ()>((js_error,))?;
            }
        }

        Ok(promise)
    }

    /// returns the body as an ArrayBuffer
    fn array_buffer<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Promise<'js>> {
        let (promise, resolve, _reject) = ctx.promise()?;
        let ab = rquickjs::ArrayBuffer::new(ctx, self.body.as_ref())?;
        resolve.call::<_, ()>((ab,))?;
        Ok(promise)
    }

    #[qjs(static, rename = "json")]
    fn new_json<'js>(
        ctx: Ctx<'js>,
        data: rquickjs::Value<'js>,
        init: Opt<rquickjs::Value<'js>>,
    ) -> JsResult<Self> {
        let data_str = ctx
            .json_stringify(data)?
            .ok_or_else(|| rquickjs::Error::new_from_js("your data", "string"))?
            .into_value();

        let mut response = Self::new(data_str, init, ctx.clone())?;

        let ctype_name = HeaderName::from_static("content-type");
        if !response.headers.contains_key(&ctype_name) {
            response
                .headers
                .insert(ctype_name, HeaderValue::from_static("application/json"));
        }

        Ok(response)
    }

    /// create a new network error response
    #[qjs(static, rename = "error")]
    fn new_error() -> Self {
        Self {
            status: 0,
            url: String::new(),
            headers: HeaderMap::new(),
            body: Bytes::new(),
            redirected: false,
        }
    }

    /// create a new redirect response
    #[qjs(static, rename = "redirect")]
    fn new_redirect(url: String, status: Opt<u16>) -> JsResult<Self> {
        let status = status.0.unwrap_or(302);

        if !matches!(status, 301 | 302 | 303 | 307 | 308) {
            return Err(rquickjs::Error::new_from_js(
                "RangeError",
                "Invalid redirect status code",
            ));
        }

        let mut headers = HeaderMap::new();
        let loc_val = HeaderValue::from_str(&url).map_err(|e| {
            // TODO: double check that new_from_js_message is being used correctly
            rquickjs::Error::new_from_js_message("TypeError", "Invalid redirect URL", e.to_string())
        })?;

        headers.insert(HeaderName::from_static("location"), loc_val);

        Ok(Self {
            status,
            url: String::new(),
            headers,
            body: Bytes::new(),
            redirected: false,
        })
    }
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

// TODO: .blob() and .formData() extractors for Request/Response? (quickjs doesnt have these built in) (necessary for compatibility but low priority)
