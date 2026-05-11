use rquickjs::{class::Trace, Ctx, JsLifetime, Result as JsResult};

/// http request object
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Request {
    url: String, // can't use url::Url here
    method: String,
    // redirect: RequestRedirect,
    // TODO: headers, body, path, statusText
}

/// http response object
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Response {
    // TODO: body, headers, status, url, statusText, bodyUsed, ok, redirected
}

/// http headers
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct Headers {
    // TODO
}

pub enum RequestRedirect {
    // TODO
    // follow, return directly, error
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl Request {
    /// the request URL
    #[qjs(get)]
    fn url(&self, _ctx: Ctx<'_>) -> JsResult<String> {
        Ok(self.url.clone())
    }

    /// the HTTP method
    #[qjs(get)]
    fn method(&self, _ctx: Ctx<'_>) -> JsResult<String> {
        Ok(self.method.clone())
    }

    /// the redirect mode
    #[qjs(get)]
    fn redirect(&self, _ctx: Ctx<'_>) -> JsResult<String> {
        todo!()
    }

    /// request headers
    #[qjs(get)]
    fn headers<'js>(&self, _ctx: Ctx<'js>) -> JsResult<rquickjs::Value<'js>> {
        todo!()
    }

    #[qjs(get)]
    fn clone<'js>(&self, _ctx: Ctx<'js>) -> JsResult<rquickjs::Value<'js>> {
        todo!()
    }

    // TODO: body retrieval: body, blob, arrayBuffer, bytes, json, text
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl Response {
    /// response body
    fn body<'js>(&self, _ctx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// response headers
    fn headers<'js>(&self, _ctx: Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    /// HTTP status code
    fn status(&self, _ctx: Ctx<'_>) -> rquickjs::Result<u16> {
        todo!()
    }

    /// response URL
    fn url(&self, _ctx: Ctx<'_>) -> rquickjs::Result<String> {
        todo!()
    }

    fn status_text(&self, _ctx: Ctx<'_>) -> rquickjs::Result<String> {
        todo!()
    }

    fn body_used(&self, _ctx: Ctx<'_>) -> rquickjs::Result<bool> {
        todo!()
    }

    fn ok(&self, _ctx: Ctx<'_>) -> rquickjs::Result<bool> {
        todo!()
    }

    fn redirected(&self, _ctx: Ctx<'_>) -> rquickjs::Result<bool> {
        todo!()
    }

    fn text(&self, _ctx: Ctx<'_>) -> rquickjs::Result<()> {
        todo!()
    }

    fn json(&self, _ctx: Ctx<'_>) -> rquickjs::Result<()> {
        todo!()
    }

    fn array_buffer(&self, _ctx: Ctx<'_>) -> rquickjs::Result<()> {
        todo!()
    }

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
    /// append a value to a header
    fn append(&self, _name: String, _value: String) {
        todo!()
    }

    /// remove a header
    fn delete(&self, _name: String) {
        todo!()
    }

    /// get a header value
    fn get(&self, _name: String) -> Option<String> {
        todo!()
    }

    /// check if a header exists
    fn has(&self, _name: String) -> bool {
        todo!()
    }

    /// set a header value
    fn set(&self, _name: String, _value: String) {
        todo!()
    }

    /// iterate over all header entries
    fn entries<'js>(&self) -> rquickjs::Value<'js> {
        todo!()
    }

    /// iterate over all header keys
    fn keys<'js>(&self) -> rquickjs::Value<'js> {
        todo!()
    }

    /// iterate over all header values
    fn values<'js>(&self) -> rquickjs::Value<'js> {
        todo!()
    }

    /// call a function for each header
    fn for_each(&self, _callback: rquickjs::Function<'_>) {
        todo!()
    }

    /// get the Set-Cookie header(s)
    fn get_set_cookie(&self) -> Vec<String> {
        todo!()
    }
}
