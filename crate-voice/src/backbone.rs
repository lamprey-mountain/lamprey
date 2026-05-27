//! connection to other sfus

use std::{net::SocketAddr, sync::Arc};

use anyhow::{anyhow, Result};
use common::v1::types::{
    voice::messages::{BackboneDatagram, BackboneDispatch, BackboneDispatchEnvelope},
    SfuId,
};
use dashmap::DashMap;
use quinn::{default_runtime, Connection, RecvStream, SendStream};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, info, trace};

use crate::{
    peer::{Command::GenerateKeyframe, CommandFull},
    sfu::State,
};

/// internal state shared across all BackboneComms handles
pub struct BackboneShared {
    /// active quic connections
    connections: DashMap<SfuId, Connection>,

    /// channels to send stuff to sfus
    control_txs: DashMap<SfuId, UnboundedSender<BackboneDispatchEnvelope>>,

    /// tokens authorized by the master for incoming connections
    pending_tokens: DashMap<String, SfuId>,
}

/// manages communication with other sfus
#[derive(Clone)]
pub struct BackboneComms {
    shared: Arc<BackboneShared>,
    endpoint: quinn::Endpoint,
    event_tx: UnboundedSender<BackboneEvent>,
}

#[derive(Debug)]
pub enum BackboneEvent {
    /// a command was received
    Dispatch {
        sfu_id: SfuId,

        /// the nonce for this dispatch, or what nonce was acked
        nonce: Option<String>,

        dispatch: BackboneDispatch,
    },

    /// a datagram was received
    Datagram(BackboneDatagram),

    /// a connection was opened
    Connected { sfu_id: SfuId },

    /// a connection was closed
    Closed { sfu_id: SfuId },
}

impl From<BackboneEvent> for Option<CommandFull> {
    fn from(value: BackboneEvent) -> Self {
        match value {
            BackboneEvent::Dispatch { dispatch, .. } => match dispatch {
                BackboneDispatch::Keyframe {
                    user_id,
                    mid,
                    rid,
                    kind,
                } => Some(CommandFull::Inner(GenerateKeyframe {
                    mid,
                    rid,
                    kind,
                    user_id,
                })),
                _ => None,
            },
            BackboneEvent::Datagram(dg) => match dg {
                BackboneDatagram::Media(media_data) => Some(CommandFull::MediaData(media_data)),
                BackboneDatagram::Speaking(speaking) => Some(CommandFull::Speaking(speaking)),
            },
            BackboneEvent::Connected { .. } => None,
            BackboneEvent::Closed { .. } => None,
        }
    }
}

impl BackboneComms {
    pub fn create(state: State) -> Result<(Self, UnboundedReceiver<BackboneEvent>)> {
        let subject_alt_names = vec!["lamprey-sfu".to_string()];
        let cert = rcgen::generate_simple_self_signed(subject_alt_names).unwrap();
        let key = rustls::pki_types::PrivateKeyDer::Pkcs8(cert.signing_key.serialize_der().into());
        let cert_der = cert.cert.der().clone();

        let certs = vec![cert_der];

        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;
        server_crypto.alpn_protocols = vec![b"lamprey-rtc".to_vec()];

        let mut config = quinn::ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?,
        ));
        let transport_config = Arc::get_mut(&mut config.transport).unwrap();
        transport_config.max_concurrent_uni_streams(0_u8.into());
        transport_config.max_concurrent_bidi_streams(1_u8.into());

        let host = state
            .voice_config
            .host_ipv4
            .clone()
            .or_else(|| state.voice_config.host_ipv6.clone())
            .unwrap_or_else(|| "0.0.0.0".to_string());

        let quic_port = state.voice_config.quic_port;
        let addr: SocketAddr = format!("{}:{}", host, quic_port).parse()?;

        let socket = std::net::UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;

        let endpoint_config = quinn::EndpointConfig::default();
        let endpoint = quinn::Endpoint::new(
            endpoint_config,
            Some(config),
            socket,
            default_runtime().unwrap(),
        )?;

        // TODO: use tokio socket
        // let socket = tokio::net::UdpSocket::bind(addr).await?;
        // let endpoint_config = quinn::EndpointConfig::default();
        // let endpoint = quinn::Endpoint::new_with_abstract_socket(
        //     endpoint_config,
        //     Some(config),
        //     Arc::new(socket), // TODO
        //     default_runtime().unwrap(),
        // )?;

        info!("listening on {}", endpoint.local_addr()?);

        let shared = Arc::new(BackboneShared {
            connections: DashMap::new(),
            control_txs: DashMap::new(),
            pending_tokens: DashMap::new(),
        });

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let me = Self {
            shared: shared.clone(),
            endpoint: endpoint.clone(),
            event_tx,
        };

        let handle = me.clone();
        tokio::spawn(async move {
            while let Some(incoming) = endpoint.accept().await {
                let h = handle.clone();
                tokio::spawn(async move {
                    if let Err(e) = h.serve_incoming(incoming).await {
                        error!("Backbone inbound connection error: {}", e);
                    }
                });
            }
        });

        Ok((me, event_rx))
    }

    pub fn add_pending_token(&mut self, token: String, expected_sfu_id: SfuId) {
        self.shared.pending_tokens.insert(token, expected_sfu_id);
    }

    pub async fn connect(&mut self, addr: SocketAddr, token: String) -> Result<()> {
        let conn = self.endpoint.connect(addr, "lamprey-rtc")?.await?;
        let (mut send, mut recv) = conn.open_bi().await?;

        // 1. send hello
        send_dispatch(
            &mut send,
            &BackboneDispatchEnvelope {
                nonce: None,
                dispatch: BackboneDispatch::Hello { token },
            },
        )
        .await?;

        // 2. recv ack
        let ack = recv_dispatch(&mut recv).await?;
        if !matches!(ack.dispatch, BackboneDispatch::Ack) {
            return Err(anyhow!("Did not receive Ack from remote SFU"));
        }

        // TODO: get actual sfu id
        let remote_sfu_id = SfuId::default();
        self.register_connection(remote_sfu_id, conn, send, recv);

        Ok(())
    }

    /// get rtt for a specific sfu
    pub fn get_rtt(&self, sfu_id: &SfuId) -> Option<std::time::Duration> {
        self.shared.connections.get(sfu_id).map(|c| c.rtt())
    }

    /// send a dispatch to a specific sfu
    pub fn send_dispatch(
        &self,
        target: SfuId,
        dispatch: BackboneDispatchEnvelope,
    ) -> Result<()> {
        if let Some(tx) = self.shared.control_txs.get(&target) {
            tx.send(dispatch)
                .map_err(|_| anyhow!("Backbone connection task died"))?;
            Ok(())
        } else {
            Err(anyhow!("no active backbone connection to sfu {}", target))
        }
    }

    /// send an unreliable datagram to a list of sfus
    pub fn broadcast_datagram(&self, destinations: &[SfuId], data: BackboneDatagram) {
        let bytes = data.to_bytes();
        for dest in destinations {
            if let Some(conn) = self.shared.connections.get(dest) {
                if let Err(e) = conn.send_datagram(bytes.clone()) {
                    trace!("failed to send backbone datagram to {}: {}", dest, e);
                }
            }
        }
    }

    async fn serve_incoming(&self, incoming: quinn::Incoming) -> Result<()> {
        let conn = incoming.await?;
        debug!("new backbone connection from {}", conn.remote_address());

        let (mut send, mut recv) = conn.accept_bi().await?;
        let msg = recv_dispatch(&mut recv).await?;

        let remote_sfu_id = match msg.dispatch {
            BackboneDispatch::Hello { token } => {
                if let Some((_, sfu_id)) = self.shared.pending_tokens.remove(&token) {
                    send_dispatch(
                        &mut send,
                        &BackboneDispatchEnvelope {
                            nonce: msg.nonce,
                            dispatch: BackboneDispatch::Ack,
                        },
                    )
                    .await?;
                    sfu_id
                } else {
                    conn.close(0u32.into(), b"invalid token");
                    return Err(anyhow!("Invalid token"));
                }
            }
            _ => {
                conn.close(0u32.into(), b"handshake expected");
                return Err(anyhow!("Handshake failed"));
            }
        };

        self.register_connection(remote_sfu_id, conn, send, recv);
        Ok(())
    }

    fn register_connection(
        &self,
        sfu_id: SfuId,
        conn: Connection,
        send: SendStream,
        recv: RecvStream,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();
        self.shared.connections.insert(sfu_id, conn.clone());
        self.shared.control_txs.insert(sfu_id, tx);

        _ = self.event_tx.send(BackboneEvent::Connected { sfu_id });
        info!("backbone connection established with SFU {}", sfu_id);

        let shared = self.shared.clone();
        let ev_tx = self.event_tx.clone();

        tokio::spawn(async move {
            run_connection_loops(sfu_id, conn, send, recv, rx, ev_tx, shared).await;
        });
    }
}

async fn run_connection_loops(
    sfu_id: SfuId,
    conn: Connection,
    mut send: SendStream,
    mut recv: RecvStream,
    mut dispatch_rx: UnboundedReceiver<BackboneDispatchEnvelope>,
    event_tx: UnboundedSender<BackboneEvent>,
    shared: Arc<BackboneShared>,
) {
    // receive datagrams
    let dgram_conn = conn.clone();
    let dgram_tx = event_tx.clone();
    let dgram_task = tokio::spawn(async move {
        while let Ok(bytes) = dgram_conn.read_datagram().await {
            if let Ok(dg) = BackboneDatagram::from_bytes(&bytes) {
                _ = dgram_tx.send(BackboneEvent::Datagram(dg));
            }
        }
    });

    // receive remote data
    let recv_tx = event_tx.clone();
    let recv_task = tokio::spawn(async move {
        while let Ok(env) = recv_dispatch(&mut recv).await {
            _ = recv_tx.send(BackboneEvent::Dispatch {
                sfu_id,
                nonce: env.nonce,
                dispatch: env.dispatch,
            });
        }
    });

    // send dispatches
    let send_task = tokio::spawn(async move {
        while let Some(env) = dispatch_rx.recv().await {
            if send_dispatch(&mut send, &env).await.is_err() {
                break;
            }
        }
    });

    // if any stream fails/closes, tear down the whole connection
    tokio::select! {
        _ = dgram_task => {},
        _ = recv_task => {},
        _ = send_task => {},
    }

    // cleanup
    shared.connections.remove(&sfu_id);
    shared.control_txs.remove(&sfu_id);
    _ = event_tx.send(BackboneEvent::Closed { sfu_id });
}

/// send length prefixed json
async fn send_dispatch(stream: &mut SendStream, dispatch: &BackboneDispatchEnvelope) -> Result<()> {
    let bytes = serde_json::to_vec(dispatch)?;
    let len = bytes.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&bytes).await?;
    Ok(())
}

/// read length prefixed json
async fn recv_dispatch(stream: &mut RecvStream) -> Result<BackboneDispatchEnvelope> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;

    let dispatch = serde_json::from_slice(&buf)?;
    Ok(dispatch)
}
