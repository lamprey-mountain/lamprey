use std::net::SocketAddr;

use common::v1::types::SfuId;
use dashmap::DashMap;
use lamprey_backend_core::config::Config;
use quinn::{ConnectionError, default_runtime};
use tokio::{sync::mpsc, task::JoinSet};
use tracing::{debug, error, info};

use crate::mesh::remote::RemoteHandle;
use crate::mesh::stream::MeshStream;
use crate::prelude::*;

// TODO: use postcard to serialize/deserialize datagrams and stream data
// postcard::from_bytes(&[1, 2, 3])
// postcard::to_stdvec(&Foo {});
mod datagram;
mod remote;
mod stream;

#[derive(Clone)]
pub struct MeshHandle {
    shared: Arc<MeshShared>,
    event_tx: mpsc::UnboundedSender<MeshEvent>,
}

/// manages QUIC connections to other SFUs
pub struct Mesh {
    shared: Arc<MeshShared>,
    endpoint: quinn::Endpoint,
    client_tasks: JoinSet<Result<()>>,
    // event_tx: mpsc::UnboundedSender<MeshEvent>,
    // event_rx: mpsc::UnboundedReceiver<MeshEvent>
}

/// internal shared state
struct MeshShared {
    /// active QUIC connections indexed by remote SFU id
    remotes: DashMap<SfuId, RemoteHandle>,

    /// tokens authorized by the master for incoming connections
    pending_tokens: DashMap<String, SfuId>,
}

#[derive(Debug)]
pub enum MeshEvent {
    // TODO
    // /// a reliable dispatch was received from a remote SFU
    // Dispatch {
    //     sfu_id: SfuId,
    //     nonce: Option<String>,
    //     dispatch: MeshDispatch,
    // },

    // /// an unreliable datagram was received from a remote SFU
    // Datagram(MeshDatagram),
    /// a mesh connection was established
    Connected { sfu_id: SfuId },

    /// a mesh connection was closed
    Closed { sfu_id: SfuId },
}

impl Mesh {
    /// create a new mesh listener and return a handle
    pub async fn spawn(config: &Config) -> Result<MeshHandle> {
        let subject_alt_names = vec!["lamprey-sfu".to_string()];
        let cert = rcgen::generate_simple_self_signed(subject_alt_names).unwrap();
        let key = rustls::pki_types::PrivateKeyDer::Pkcs8(cert.signing_key.serialize_der().into());
        let cert_der = cert.cert.der().clone();

        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key)?;
        server_crypto.alpn_protocols = vec![b"lamprey-rtc".to_vec()];

        let voice_config = config.voice.as_ref().expect("TODO: better error handling");

        let mut quic_config = quinn::ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
                .expect("TODO: better error handling"),
        ));
        let transport_config = Arc::get_mut(&mut quic_config.transport).unwrap();
        transport_config.max_concurrent_uni_streams(0_u8.into());
        transport_config.max_concurrent_bidi_streams(1_u8.into());

        let addr_v4: SocketAddr = format!(
            "{}:{}",
            voice_config.host_ipv4.as_deref().unwrap_or("0.0.0.0"),
            voice_config.quic_port
        )
        .parse()
        .expect("TODO: better error handling");
        let socket_v4 = std::net::UdpSocket::bind(addr_v4)?;
        socket_v4.set_nonblocking(true)?;
        // TODO: listen on v4 and v6

        let endpoint = quinn::Endpoint::new(
            quinn::EndpointConfig::default(),
            Some(quic_config),
            socket_v4,
            default_runtime().unwrap(),
        )?;

        info!("Mesh listening on {}", endpoint.local_addr()?);

        let shared = Arc::new(MeshShared {
            remotes: DashMap::new(),
            pending_tokens: DashMap::new(),
        });

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let me = Mesh {
            shared: shared.clone(),
            endpoint,
            client_tasks: JoinSet::new(),
        };

        let handle = MeshHandle { shared, event_tx };

        tokio::spawn(me.run());

        Ok(handle)
    }

    async fn run(mut self) {
        loop {
            tokio::select! {
                Some(incoming) = self.endpoint.accept() => {
                    self.handle_incoming(incoming);
                }
                Some(next) = self.client_tasks.join_next() => {
                    match next {
                        Err(err) => { error!("join error {err}") }
                        Ok(Err(err)) => { error!("client error {err}") }
                        Ok(Ok(())) => {}
                    }
                }
            }
        }
    }

    #[tracing::instrument(skip(self, incoming))]
    async fn handle_incoming(&mut self, incoming: quinn::Incoming) {
        let shared = Arc::clone(&self.shared);

        self.client_tasks.spawn(async move {
            let conn = incoming.await?;
            debug!("new mesh connection from {}", conn.remote_address());

            loop {
                let (send, recv) = match conn.accept_bi().await {
                    Ok(conn) => conn,
                    Err(err) => match err {
                        ConnectionError::ApplicationClosed(c) => {
                            debug!("closed with code {}", c.error_code);
                            break;
                        }
                        ConnectionError::LocallyClosed => break,
                        err => return Err(err.into()),
                    },
                };

                let stream = MeshStream::new(send, recv)
                    .accept()
                    .await
                    .expect("TODO: better error handling");
                // match stream { }
                // conn.close(error_code, reason);
                // recv.read(buf).await.unwrap();
                // send.write(buf).await.unwrap();

                // let handle: ShardHandle = todo!();
                // handle.handle_remote_inbound(stream);

                // fn handle_remote_inbound(&self, stream: Stream<Subscription>) {
                //     // poll stream, forward frames to call
                //     // maybe spawn a tokio task? (who has the JoinSet)
                // }

                todo!()
            }

            Ok(())
        });
    }
}

impl MeshHandle {
    /// initiate an outbound connection to a remote SFU
    pub async fn connect(
        &self,
        addr: SocketAddr,
        token: String,
        remote_sfu_id: SfuId,
    ) -> Result<()> {
        todo!()
    }

    /// get a handle to a peer
    pub fn lookup(&self, sfu_id: &SfuId) -> Option<RemoteHandle> {
        todo!()
    }

    /// register a token for an expected incoming connection
    pub fn add_pending_token(&self, token: String, expected_sfu_id: SfuId) {
        todo!()
    }

    pub fn subscribe(&self) -> impl Stream<Item = MeshEvent> {
        // TODO
        futures_util::stream::empty().boxed()
    }
}
