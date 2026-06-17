use rquickjs::{Ctx, JsLifetime, Result as JsResult, class::Trace};

/// network manager for making HTTP requests and future protocols
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct NetworkManager {
    // TODO
}

/// opaque container representing an IP address
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct IpAddress {
    // TODO: store actual IP address data
}

// pub struct TcpConnection {
//     // TODO
// }

// pub struct QuicConnection {
//     // TODO
// }

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl NetworkManager {
    #[qjs(constructor)]
    fn new() -> JsResult<Self> {
        Err(rquickjs::Error::new_from_js(
            "Request",
            "Can't manually construct this!",
        ))
    }

    /// make an http request
    fn fetch<'js>(
        &self,
        _req: rquickjs::Object<'js>,
        _ctx: Ctx<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        todo!()
    }

    // TODO: connect_tcp
    // TODO: connect_quic
    // NOTE: connect_tls probably won't exist, a tls module could exist instead
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl IpAddress {
    /// compare two IP addresses for equality
    fn equals(&self, _other: rquickjs::Object<'_>) -> bool {
        todo!()
    }
}

#[rquickjs::module(rename = "lamprey:net")]
pub mod inner {
    pub use super::{IpAddress, NetworkManager};
}
