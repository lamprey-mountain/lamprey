use bytes::Bytes;
use http::HeaderMap;
use rquickjs::{
    Ctx, JsLifetime, Result as JsResult,
    class::{Trace, Tracer},
};

use crate::javascript::glue::http::Headers;

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

impl<'js> Trace<'js> for Request {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
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
