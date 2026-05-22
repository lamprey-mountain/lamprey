//! connection to other sfus

use std::{net::SocketAddr, sync::Arc};

use anyhow::anyhow;
use common::v1::types::{
    voice::messages::{BackboneDatagram, BackboneDispatch, BackboneDispatchEnvelope},
    SfuId,
};
use dashmap::DashMap;
use quinn::{default_runtime, RecvStream, SendStream};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, info, trace, warn};

use crate::{error::Result, sfu::State};

/// manages communication with other sfus
pub struct BackboneComms {
    endpoint: quinn::Endpoint,

    /// tokens authorized by the Master for incoming connections
    pending_tokens: DashMap<String, SfuId>,

    internal_rx: UnboundedReceiver<BackboneEvent>,
    internal_tx: UnboundedSender<BackboneEvent>,
}

#[derive(Debug)]
pub enum BackboneEvent {
    /// a command was received was closed
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

impl BackboneComms {
    pub fn create(state: State) -> Result<Self> {
        //     let certs = todo!("generate on startup");
        //     let key = todo!("generate on startup");
        let addr = todo!("use voice_config host_ipv4/host_ipv6 + quic_port");
        let socket = todo!("open socket");

        //     let mut server_crypto = rustls::ServerConfig::builder()
        //         .with_no_client_auth()
        //         .with_single_cert(certs, key)?;
        //     server_crypto.alpn_protocols = vec![b"lamprey-rtc".to_vec()];

        //     let mut config = quinn::ServerConfig::with_crypto(Arc::new(
        //         quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?,
        //     ));
        //     let transport_config = Arc::get_mut(&mut config.transport).unwrap();
        //     transport_config.max_concurrent_uni_streams(0.into());
        //     transport_config.max_concurrent_bidi_streams(1.into());

        // let config = quinn::EndpointConfig::new(todo!());
        //     let quic = quinn::Endpoint::new(config, server_config, socket, runtime)
        //         // let rng = &mut rand::rng();
        //         // let mut master_key = [0u8; 64];
        //         // rng.fill_bytes(&mut master_key);
        //         // let master_key = hkdf::Salt::new(hkdf::HKDF_SHA256, &[]).extract(&master_key);
        //         // Self::new(crypto, Arc::new(master_key))

        let config = todo!();
        let server_config = todo!();
        let endpoint = quinn::Endpoint::new(config, server_config, socket, default_runtime())?;
        info!("listening on {}", endpoint.local_addr()?);

        let pending_tokens = Arc::new(DashMap::new());
        let (internal_tx, internal_rx) = mpsc::unbounded_channel();

        let endpoint2 = endpoint.clone();
        let pending_tokens2 = Arc::clone(&pending_tokens);
        tokio::spawn(async move {
            while let Some(incoming) = endpoint2.accept().await {
                let tx = internal_tx.clone();

                tokio::spawn(async move {
                    if let Err(e) = serve(incoming, tx, &mut pending_tokens2).await {
                        error!("Backbone inbound connection error: {}", e);
                    }
                });
            }

            Ok(())
        });

        Ok(Self {
            endpoint,
            pending_tokens,
            internal_rx,
            internal_tx,
        })
    }

    pub fn add_pending_token(&mut self, token: String, expected_sfu_id: SfuId) {
        self.pending_tokens.insert(token, expected_sfu_id);
    }

    /// poll all active connections
    pub async fn poll(&self) -> BackboneEvent {
        self.internal_rx.recv().await?
    }

    pub async fn connect(&self, addr: SocketAddr, token: String) -> Result<()> {
        let conn = self.endpoint.connect(addr, "lamprey-rtc")?.await?;
        let (mut send, mut recv) = conn.open_bi().await?;

        // 1. send hello
        // let nonce = todo!();
        send_dispatch(
            &mut send,
            &BackboneDispatchEnvelope {
                // nonce: Some(nonce),
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

        // conn.read_datagram().await.unwrap();
        // conn.max_datagram_size();
        // conn.send_datagram(BackboneDatagram::Speaking(_).to_bytes())
        //     .unwrap();

        // conn.rtt();

        // send.write_all(buf);
        // recv.read(buf);

        Ok(())
    }

    /// send a dispatch to a specific sfu
    pub async fn send_dispatch(
        &mut self,
        target: SfuId,
        dispatch: &BackboneDispatchEnvelope,
    ) -> Result<()> {
        if let Some(stream) = self.control_channels.get_mut(&target) {
            send_dispatch(stream, dispatch).await
        } else {
            Err(anyhow!("no active backbone connection to sfu {}", target))
        }
    }

    /// send an unreliable datagram to a list of sfus
    pub fn broadcast_datagram(&self, destinations: &[SfuId], data: BackboneDatagram) {
        let bytes = data.to_bytes();
        for dest in destinations {
            if let Some(conn) = self.connections.get(dest) {
                if let Err(e) = conn.send_datagram(bytes.clone()) {
                    trace!("failed to send backbone datagram to {}: {}", dest, e);
                }
            }
        }
    }
}

async fn serve(
    incoming: quinn::Incoming,
    event_tx: UnboundedSender<BackboneEvent>,
    pending_tokens: (),
) -> Result<()> {
    let conn = incoming.await?;
    debug!("New backbone connection from {}", conn.remote_address());

    // 1. handshake
    let (mut send, mut recv) = conn.accept_bi().await?;
    let msg = recv_dispatch(&mut recv).await?;
    let remote_sfu_id = match msg.dispatch {
        BackboneDispatch::Hello { token } => {
            if let Some(sfu_id) = pending_tokens.remove(&token) {
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
                warn!("Backbone connection rejected: Invalid token");
                conn.close(0u32.into(), b"invalid token");
                return Err(anyhow!("Invalid token"));
            }
        }
        _ => {
            conn.close(0u32.into(), b"handshake expected");
            return Err(anyhow!("Handshake failed"));
        }
    };

    info!("Backbone connection established with SFU {}", remote_sfu_id);

    // TODO: notify manager of new connection
    _ = event_tx.send(BackboneEvent::Connected {
        sfu_id: remote_sfu_id,
    });

    let mut datagrams = conn.datagrams();

    loop {
        tokio::select! {
            res = recv_dispatch(&mut recv) => {
                match res {
                    Ok(dispatch) => {
                        _ = event_tx.send(BackboneEvent::Dispatch{ sfu_id: todo!(), nonce: todo!(), dispatch: todo!() });
                    }
                    // TODO: log error
                    Err(_) => break,
                }
            }
            Some(res) = datagrams.next() => {
                match res {
                    Ok(bytes) => {
                        if let Ok(dg) = BackboneDatagram::from_bytes(&bytes) {
                            _ = event_tx.send(BackboneEvent::Datagram(dg));
                        }
                    }
                    // TODO: log error
                    Err(_) => break,
                }
            }
        }
    }

    _ = event_tx.send(BackboneEvent::Closed {
        sfu_id: remote_sfu_id,
    });

    Ok(())
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
