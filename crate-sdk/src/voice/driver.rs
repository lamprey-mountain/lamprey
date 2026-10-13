use std::{sync::Arc, time::Instant};

use common::{
    v1::types::{
        MessageClient, MessageSync,
        voice::{
            SessionDescription, VoiceStateUpdate,
            messages::{SignallingCommand, SignallingEvent},
        },
    },
    v2::types::ChannelId,
};
use str0m::{Candidate, Rtc};
use tokio::{sync::mpsc, time};
use tracing::{debug, error, info};

use crate::{
    cache::Cache,
    syncer::SyncerHandle,
    voice::{
        VoiceError,
        client::{ConnectionState, LocalVoiceState, Shared},
        track::Inbound,
    },
};

/// a command to control the rtc worker
#[derive(Debug, Clone)]
pub(crate) enum RtcCommand {
    /// handle a local ice candidate
    Candidate(Candidate),

    /// mute local audio
    Mute(bool),

    /// deafen local audio
    Deafen(bool),
}

pub struct Driver {
    rtc: Rtc,
    rx: mpsc::Receiver<RtcCommand>,
    syncer: SyncerHandle,
    cache: Cache,
    shared: Arc<Shared>,
    channel_id: ChannelId,
    pending: Option<str0m::change::SdpPendingOffer>,
}

impl Driver {
    pub fn new(
        rtc: Rtc,
        rx: mpsc::Receiver<RtcCommand>,
        syncer: SyncerHandle,
        cache: Cache,
        shared: Arc<Shared>,
        channel_id: ChannelId,
    ) -> Self {
        Driver {
            rtc,
            rx,
            syncer,
            cache,
            shared,
            channel_id,
            pending: None,
        }
    }

    pub async fn spawn(mut self) {
        loop {
            if let Err(e) = self.step().await {
                error!("rtc step error: {e}");
            }
        }
    }

    async fn step(&mut self) -> Result<(), VoiceError> {
        if !self.rtc.is_alive() {
            // TODO: handle rtc dead
            error!("rtc dead");
            return Err(VoiceError::Dead);
        }

        let output = match self.rtc.poll_output() {
            Ok(o) => o,
            Err(e) => {
                error!("rtc poll error: {e}");
                return Err(VoiceError::Rtc(e));
            }
        };

        let timeout = match output {
            str0m::Output::Timeout(instant) => instant,
            str0m::Output::Transmit(v) => {
                self.shared.sock.send_to(&v.contents, v.destination).await?;
                return Ok(());
            }
            str0m::Output::Event(event) => {
                self.handle_str0m_event(event).await?;
                return Ok(());
            }
        };

        let mut packet_buf = vec![0; 2048];
        let sleep = time::sleep_until(time::Instant::from_std(timeout));

        tokio::select! {
            biased;

            Some(cmd) = self.rx.recv() => {
                self.handle_command(cmd).await ?;
                return Ok(())
            },

            // TODO: handle err lagged
            Ok(sync) = self.syncer.poll_sync() => {
                self.handle_sync(sync).await?;
                return Ok(())
            },

            Ok((n, source)) = self.shared.sock.recv_from(&mut packet_buf) => {
                let res = self.rtc.handle_input(str0m::Input::Receive(
                    Instant::now(),
                    str0m::net::Receive {
                        proto: str0m::net::Protocol::Udp,
                        source,
                        destination: self.shared.sock.local_addr()?,
                        contents: packet_buf[..n].try_into()?,
                    },
                ));
                if let Err(e) = res {
                    error!("rtc handle_input error: {e}");
                }
            }

            _ = sleep => {
                if let Err(e) = self.rtc.handle_input(str0m::Input::Timeout(Instant::now())) {
                    error!("rtc handle_input timeout error: {e}");
                    // TODO: what now?
                }
            },
        }

        Ok(())
    }

    async fn handle_sync(&mut self, sync: Arc<MessageSync>) -> Result<(), VoiceError> {
        match &*sync {
            MessageSync::VoiceDispatch {
                payload,
                channel_id,
                ..
            } if channel_id == &self.channel_id => match payload {
                SignallingEvent::Connected { .. } => {
                    info!("Connected to SFU");
                    self.shared.conn_state.send_modify(|state| {
                        if matches!(
                            *state,
                            ConnectionState::Disconnected
                                | ConnectionState::Pending
                                | ConnectionState::Failed
                        ) {
                            *state = ConnectionState::Checking;
                        }
                    });
                }
                SignallingEvent::Disconnected => {
                    info!("Disconnected from SFU");
                    self.shared
                        .conn_state
                        .send_replace(ConnectionState::Disconnected);
                }
                SignallingEvent::Offer { sdp, .. } => {
                    let sdp = str0m::change::SdpOffer::from_sdp_string(&sdp.0)?;
                    let answer = self.rtc.sdp_api().accept_offer(sdp)?;
                    self.syncer.send(MessageClient::VoiceDispatch {
                        channel_id: self.channel_id,
                        nonce: None,
                        command: SignallingCommand::Answer {
                            sdp: SessionDescription(answer.to_sdp_string()),
                        },
                    });
                }
                SignallingEvent::Answer { sdp } => {
                    let sdp = str0m::change::SdpAnswer::from_sdp_string(&sdp.0)?;
                    if let Some(pending) = self.pending.take() {
                        self.rtc.sdp_api().accept_answer(pending, sdp)?;
                    } else {
                        error!("got answer without a pending offer, ignoring");
                    }
                }
                SignallingEvent::Candidate { candidate } => {
                    if let Ok(c) = str0m::Candidate::from_sdp_string(&candidate.0) {
                        self.rtc.add_remote_candidate(c);
                    } else {
                        error!("failed to parse candidate: {}", candidate.0);
                    }
                }
                SignallingEvent::Tracks {
                    user_id,
                    added,
                    removed,
                } => {
                    for track in added {
                        let inbound = Inbound {
                            id: track.id,
                            user_id: *user_id,
                            kind: track.inner.kind,
                            key: track.inner.key.clone(),
                        };
                        self.shared.tracks.inbound.insert(track.id, inbound);
                    }
                    for track_id in removed {
                        self.shared.tracks.inbound.remove(&track_id);
                    }
                }
                SignallingEvent::Subscribe(_subs) => {
                    // TODO: handle subscribe
                    // subs.add;
                    // subs.remove;
                }
                SignallingEvent::Migrate { new_sfu_id } => {
                    info!("Migrating to SFU: {:?}", new_sfu_id);
                    // TODO: handle migrate
                }
                SignallingEvent::Error { message, code } => {
                    error!("Signalling error: {} ({:?})", message, code);
                }
            },
            MessageSync::VoiceState {
                user_id,
                state,
                old_state: _,
            } => {
                if self.cache.current_user_id() == Some(*user_id) {
                    if let Some(state) = state {
                        if state.channel_id == self.channel_id {
                            self.shared.voice_state.send_modify(|local| {
                                *local = LocalVoiceState::Synced(state.clone())
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_command(&mut self, cmd: RtcCommand) -> Result<(), VoiceError> {
        match cmd {
            RtcCommand::Candidate(c) => {
                self.rtc.add_local_candidate(c);
            }
            RtcCommand::Mute(mute) => {
                let update = {
                    let state = self.shared.voice_state.borrow();
                    match &*state {
                        LocalVoiceState::Connecting(u) => VoiceStateUpdate {
                            self_mute: mute,
                            ..u.clone()
                        },
                        LocalVoiceState::Pending(_s, u) => VoiceStateUpdate {
                            self_mute: mute,
                            ..u.clone()
                        },
                        LocalVoiceState::Synced(s) => VoiceStateUpdate {
                            channel_id: s.channel_id,
                            self_deaf: s.self_deaf,
                            self_mute: mute,
                            self_video: s.self_video,
                            screenshare: None,
                        },
                    }
                };

                self.syncer.send(MessageClient::VoiceDispatch {
                    channel_id: self.channel_id,
                    nonce: None,
                    command: SignallingCommand::VoiceState {
                        state: update.clone(),
                    },
                });

                self.shared.voice_state.send_modify(|local| {
                    *local = match local {
                        LocalVoiceState::Connecting(_) => LocalVoiceState::Connecting(update),
                        LocalVoiceState::Pending(s, _) | LocalVoiceState::Synced(s) => {
                            // PERF: surely theres a way to do this without cloning
                            LocalVoiceState::Pending(s.clone(), update)
                        }
                    };
                });
            }
            RtcCommand::Deafen(deafen) => {
                let update = {
                    let state = self.shared.voice_state.borrow();
                    match &*state {
                        LocalVoiceState::Connecting(u) => VoiceStateUpdate {
                            self_deaf: deafen,
                            ..u.clone()
                        },
                        LocalVoiceState::Pending(_s, u) => VoiceStateUpdate {
                            self_deaf: deafen,
                            ..u.clone()
                        },
                        LocalVoiceState::Synced(s) => VoiceStateUpdate {
                            channel_id: s.channel_id,
                            self_deaf: deafen,
                            self_mute: s.self_mute,
                            self_video: s.self_video,
                            screenshare: None,
                        },
                    }
                };

                self.syncer.send(MessageClient::VoiceDispatch {
                    channel_id: self.channel_id,
                    nonce: None,
                    command: SignallingCommand::VoiceState {
                        state: update.clone(),
                    },
                });

                self.shared.voice_state.send_modify(|local| {
                    *local = match local {
                        LocalVoiceState::Connecting(_) => LocalVoiceState::Connecting(update),
                        LocalVoiceState::Pending(s, _) | LocalVoiceState::Synced(s) => {
                            // PERF: surely theres a way to do this without cloning
                            LocalVoiceState::Pending(s.clone(), update)
                        }
                    };
                });
            }
        }
        Ok(())
    }

    async fn handle_str0m_event(&mut self, event: str0m::Event) -> Result<(), VoiceError> {
        match event {
            str0m::Event::Connected => info!("rtc connected!"),
            str0m::Event::IceConnectionStateChange(state) => {
                debug!("ICE state changed to: {:?}", state);
                let conn_state = match state {
                    str0m::IceConnectionState::New => return Ok(()),
                    str0m::IceConnectionState::Checking => ConnectionState::Checking,
                    str0m::IceConnectionState::Connected => ConnectionState::Connected,
                    str0m::IceConnectionState::Completed => ConnectionState::Ready,
                    str0m::IceConnectionState::Disconnected => ConnectionState::Disconnected,
                };
                self.shared.conn_state.send_replace(conn_state);
            }

            // media
            // str0m::Event::MediaAdded(_) => todo!(),
            // str0m::Event::MediaData(_) => todo!(),
            // str0m::Event::KeyframeRequest(_) => todo!(),

            // datachannels
            // str0m::Event::ChannelOpen(_) => todo!(),
            // str0m::Event::ChannelData(_) => todo!(),
            // str0m::Event::ChannelClose(_) => todo!(),
            _ => {}
        }

        Ok(())
    }
}
