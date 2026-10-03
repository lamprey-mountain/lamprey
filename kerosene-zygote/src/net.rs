//! networking utils for quic

// TODO: what exactly would go here? quinn utils?
// quiche (from cloudflare) also looks interesting, but its very low level so its potentially very painful

use std::sync::Arc;

use quinn::{ServerConfig, crypto::rustls::QuicServerConfig};
use rustls::pki_types::PrivateKeyDer;

/// utility to setup quic crypto
#[derive(Debug, Default)]
pub struct Crypto {
    subject_alt_names: Vec<String>,
    alpn_protocols: Vec<Vec<u8>>,
}

impl Crypto {
    /// this key/cert is valid for this subject
    ///
    /// usually an ip address or hostname
    pub fn subject_alt_name(mut self, name: impl Into<String>) -> Self {
        self.subject_alt_names.push(name.into());
        self
    }

    /// we support this protocol
    ///
    /// for application layer protocol negotiation
    pub fn alpn_protocol(mut self, proto: impl Into<Vec<u8>>) -> Self {
        self.alpn_protocols.push(proto.into());
        self
    }

    // TODO: better error handling, don't panic
    pub fn build(self) -> Result<QuicServerConfig, ()> {
        let cert = rcgen::generate_simple_self_signed(self.subject_alt_names).unwrap();
        let key = PrivateKeyDer::Pkcs8(cert.signing_key.serialize_der().into());
        let cert_der = cert.cert.der().clone();

        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key)
            .unwrap();
        server_crypto.alpn_protocols = self.alpn_protocols;

        let config =
            QuicServerConfig::try_from(server_crypto).expect("TODO: better error handling");

        Ok(config)
    }
}

async fn example() {
    let crypto = Crypto::default()
        .subject_alt_name("localhost")
        .alpn_protocol("lamprey/sync/v1")
        // .alpn_protocol("h3") // for http3/webtransport
        .build()
        .unwrap();

    let mut config = ServerConfig::with_crypto(Arc::new(crypto));
    let transport_config = Arc::get_mut(&mut config.transport).unwrap();
    transport_config.max_concurrent_uni_streams(0u8.into());
    transport_config.max_concurrent_bidi_streams(255u8.into());

    // NOTE: do i need two endpoints/servers? one for ipv4, one for ipv6?
    let addr = "0.0.0.0:4433".parse().unwrap();
    // let addr = "[::]:4433".parse().unwrap();
    let server = quinn::Endpoint::server(config, addr).unwrap();

    while let Some(incoming) = server.accept().await {
        tokio::spawn(async move {
            let conn = incoming.await.unwrap();
            let stream = conn.accept_bi().await;
            futures::select! {}
            tokio::select! {}
        });
    }
}
