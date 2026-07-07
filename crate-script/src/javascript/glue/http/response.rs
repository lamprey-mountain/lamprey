use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue};
use rquickjs::{
    Ctx, FromJs, JsLifetime, Result as JsResult,
    class::{Trace, Tracer},
    prelude::Opt,
};

use crate::javascript::glue::http::Headers;

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

impl<'js> Trace<'js> for Response {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
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
