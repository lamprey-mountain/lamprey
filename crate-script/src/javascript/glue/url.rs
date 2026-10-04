use rquickjs::{
    Class, Coerced, Ctx, Exception, JsLifetime, Result as JsResult, Value,
    class::{Trace, Tracer},
    function::Opt,
};
use url::{ParseError, Url};

/// uniform resource locator
#[rquickjs::class(rename = "URL")]
#[derive(Clone, JsLifetime)]
pub struct JsUrl {
    url: Url,
}

impl<'js> Trace<'js> for JsUrl {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

#[derive(thiserror::Error, Debug)]
enum UrlParseError {
    #[error("Invalid base URL: {0}")]
    Base(ParseError),

    #[error("Invalid URL: {0}")]
    Url(ParseError),
}

fn parse_url(url: &str, base: Option<&str>) -> Result<Url, UrlParseError> {
    let base = base
        .map(Url::parse)
        .transpose()
        .map_err(UrlParseError::Base)?;
    Url::options()
        .base_url(base.as_ref())
        .parse(url)
        .map_err(UrlParseError::Url)
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl<'js> JsUrl {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'js>, url: Coerced<String>, base: Opt<Coerced<String>>) -> JsResult<Self> {
        let base = base.0.map(|b| b.0);
        parse_url(&url.0, base.as_deref())
            .map(|url| Self { url })
            .map_err(|e| Exception::throw_type(&ctx, &e.to_string()))
    }

    #[qjs(static)]
    fn parse(
        ctx: Ctx<'js>,
        url: Coerced<String>,
        base: Opt<Coerced<String>>,
    ) -> JsResult<Value<'js>> {
        let base = base.0.map(|b| b.0);
        match parse_url(&url.0, base.as_deref()) {
            Ok(url) => Ok(Class::instance(ctx, Self { url })?.into_value()),
            Err(_) => Ok(Value::new_null(ctx)),
        }
    }

    #[qjs(static)]
    fn can_parse(url: Coerced<String>, base: Opt<Coerced<String>>) -> bool {
        let base = base.0.map(|b| b.0);
        parse_url(&url.0, base.as_deref()).is_ok()
    }

    #[qjs(get)]
    fn origin(&self) -> String {
        self.url.origin().ascii_serialization()
    }

    #[qjs(get)]
    fn protocol(&self) -> String {
        format!("{}:", self.url.scheme())
    }

    #[qjs(set, rename = "protocol")]
    fn set_protocol(&mut self, protocol: String) {
        let scheme = protocol
            .split(':')
            .next()
            .expect("split always has at least one segment");
        _ = self.url.set_scheme(scheme);
    }

    #[qjs(get)]
    fn username(&self) -> &str {
        self.url.username()
    }

    #[qjs(set, rename = "username")]
    fn set_username(&mut self, username: String) {
        _ = self.url.set_username(&username);
    }

    #[qjs(get)]
    fn password(&self) -> &str {
        self.url.password().unwrap_or("")
    }

    #[qjs(set, rename = "password")]
    fn set_password(&mut self, password: String) {
        _ = self
            .url
            .set_password(Some(&*password).filter(|p| !p.is_empty()));
    }

    #[qjs(get)]
    fn pathname(&self) -> &str {
        self.url.path()
    }

    #[qjs(set, rename = "pathname")]
    fn set_pathname(&mut self, pathname: String) {
        self.url.set_path(&pathname);
    }

    #[qjs(get)]
    fn host(&self) -> String {
        self.url
            .host_str()
            .map(|h| {
                if let Some(port) = self.url.port() {
                    format!("{h}:{port}")
                } else {
                    h.to_string()
                }
            })
            .unwrap_or_default()
    }

    #[qjs(set, rename = "host")]
    fn set_host(&mut self, host: String) {
        // handle ipv6
        let start = host.rfind(']').unwrap_or(0);
        let (h, port) = match host[start..].find(':') {
            Some(i) => (&host[..start + i], Some(host[start + i + 1..].to_string())),
            None => (&host[..], None),
        };

        // set both hostname and port
        if self.url.set_host(Some(h)).is_ok() {
            if let Some(p) = port.filter(|p| !p.is_empty()) {
                self.set_port(Coerced(p));
            }
        }
    }

    #[qjs(get)]
    fn hostname(&self) -> &str {
        self.url.host_str().unwrap_or("")
    }

    #[qjs(set, rename = "hostname")]
    fn set_hostname(&mut self, hostname: String) {
        _ = self.url.set_host(Some(&hostname));
    }

    #[qjs(get)]
    fn port(&self) -> String {
        self.url.port().map(|p| p.to_string()).unwrap_or_default()
    }

    #[qjs(set, rename = "port")]
    fn set_port(&mut self, port: Coerced<String>) {
        let port = port.0;
        let port_num = port
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();

        if port_num.is_empty() {
            if port.is_empty() {
                _ = self.url.set_port(None);
            }
        } else if let Ok(port) = port_num.parse::<u16>() {
            _ = self.url.set_port(Some(port));
        }
    }

    #[qjs(get)]
    fn search(&self) -> String {
        self.url
            .query()
            .filter(|q| !q.is_empty())
            .map(|q| format!("?{q}"))
            .unwrap_or_default()
    }

    #[qjs(set, rename = "search")]
    fn set_search(&mut self, search: String) {
        if search.is_empty() {
            self.url.set_query(None);
        } else {
            self.url
                .set_query(Some(search.strip_prefix('?').unwrap_or(&search)));
        }
    }

    #[qjs(get)]
    fn hash(&self) -> String {
        self.url
            .fragment()
            .filter(|f| !f.is_empty())
            .map(|f| format!("#{f}"))
            .unwrap_or_default()
    }

    #[qjs(set, rename = "hash")]
    fn set_hash(&mut self, hash: String) {
        if hash.is_empty() {
            self.url.set_fragment(None);
        } else {
            self.url
                .set_fragment(Some(hash.strip_prefix('#').unwrap_or(&hash)));
        }
    }

    #[qjs(get)]
    fn href(&self) -> String {
        self.url.to_string()
    }

    #[qjs(set, rename = "href")]
    fn set_href(&mut self, ctx: Ctx<'js>, href: String) -> JsResult<()> {
        self.url =
            parse_url(&href, None).map_err(|e| Exception::throw_type(&ctx, &e.to_string()))?;
        Ok(())
    }

    fn to_string(&self) -> String {
        self.url.to_string()
    }

    #[qjs(rename = "toJSON")]
    fn to_json(&self) -> String {
        self.url.to_string()
    }
}

// TODO: impl URLSearchParams, URLPattern
// TODO: verify that this module is spec compliant
// see https://url.spec.whatwg.org/ and https://urlpattern.spec.whatwg.org/
