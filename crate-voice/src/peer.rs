use std::{collections::HashMap, sync::Arc};

use common::{
    v1::types::voice::{
        internal::SfuPermissions,
        messages::{SignallingCommand, SignallingEvent},
        KeyframeRequestKind, SessionDescription, VoiceState,
    },
    v2::types::{SfuId, UserId},
};
use str0m::Rtc;
use tracing::warn;

use crate::{
    prelude::*,
    util::{
        permissions::Permissions, signalling::Signalling, SfuVoiceState, Subscriptions, TrackId,
    },
};

/// a webrtc connection
// NOTE: do i want to make these fields pub?
pub struct Peer {
    pub kind: PeerKind,
    pub rtc: Rtc,
    pub signalling: Signalling,
    pub subscriptions: Subscriptions,

    pub track_map: HashMap<SMid, TrackId>,
    pub mid_map: HashMap<TrackId, SMid>,

    /// datachannel for speaking/voice activity messages
    ///
    /// users send `Speaking` to sfus. sfus send `SpeakingWithUserId` to each other and users.
    pub speaking_chan: Option<SChannelId>,
}

/// whos on the other end of this connection
pub enum PeerKind {
    /// an end user
    User {
        voice_state: VoiceState,
        permissions: SfuPermissions,
    },

    /// a cascading peer that bridges this shard to a remote SFU
    Cascade { remote_sfu: SfuId },
}

impl Peer {
    pub fn new(ty: PeerKind, rtc: Rtc) -> Self {
        Self {
            kind: ty,
            rtc,
            signalling: Signalling::new(),
            subscriptions: Subscriptions::default(),
            track_map: HashMap::new(),
            mid_map: HashMap::new(),
            speaking_chan: None,
        }
    }

    pub fn new_user(vs: SfuVoiceState, rtc: Rtc) -> Self {
        Self::new(
            PeerKind::User {
                voice_state: vs.inner,
                permissions: vs.permissions,
            },
            rtc,
        )
    }

    pub fn new_cascade(sfu_id: SfuId, rtc: Rtc) -> Self {
        Self::new(PeerKind::Cascade { remote_sfu: sfu_id }, rtc)
    }

    pub fn kind(&self) -> &PeerKind {
        &self.kind
    }

    pub fn user_id(&self) -> Option<UserId> {
        match &self.kind {
            PeerKind::User { voice_state, .. } => Some(voice_state.user_id),
            PeerKind::Cascade { .. } => None,
        }
    }

    /// get a track id from this peer's local mid
    pub fn lookup_track(&self, mid: SMid) -> Option<TrackId> {
        self.track_map.get(&mid).copied()
    }

    pub fn write_media(&mut self, track_id: TrackId, media: &str0m::media::MediaData) {
        let Some(mid) = self.mid_map.get(&track_id) else {
            return;
        };

        let Some(writer) = self.rtc.writer(*mid) else {
            return;
        };

        if let Some(pt) = writer.match_params(media.params) {
            let _ = writer.write(pt, media.network_time, media.time, Arc::clone(&media.data));
        }
    }

    pub fn request_keyframe(
        &mut self,
        track_id: TrackId,
        rid: Option<SRid>,
        kind: KeyframeRequestKind,
    ) {
        let Some(mid) = self.mid_map.get(&track_id) else {
            return;
        };

        if let Some(mut w) = self.rtc.writer(*mid) {
            let _ = w.request_keyframe(rid.map(Into::into), kind.into());
        }
    }

    /// handle a signalling command, returning any emitted signalling events
    pub fn handle_signalling(&mut self, cmd: SignallingCommand) -> Vec<SignallingEvent> {
        match cmd {
            SignallingCommand::Disconnect => self.rtc.disconnect(),
            SignallingCommand::VoiceState { state } => match &mut self.kind {
                PeerKind::User { voice_state, .. } => voice_state.apply(state),
                PeerKind::Cascade { .. } => warn!("got voice state update for cascade?"),
            },
            SignallingCommand::Offer { sdp, tracks: _ } => {
                match self.signalling.handle_offer(&mut self.rtc, sdp) {
                    Ok(answer) => {
                        return vec![SignallingEvent::Answer {
                            sdp: SessionDescription(answer.to_sdp_string()),
                        }]
                    }
                    Err(e) => {
                        warn!("Failed to handle offer: {:?}", e);
                    }
                }
            }
            SignallingCommand::Answer { sdp } => {
                if let Err(e) = self.signalling.handle_answer(&mut self.rtc, sdp) {
                    warn!("Failed to handle answer: {:?}", e);
                }

                // TODO: mark tracks as ready
            }
            SignallingCommand::Candidate { candidate } => {
                // TODO: ice candidates
                warn!("ignoring candidate: {:?}", candidate);
            }
            SignallingCommand::Subscribe {
                subs: subscriptions,
            } => {
                // TODO: handle subscriptions
                warn!("ignoring subscribe: {:?}", subscriptions);
            }
        }

        vec![]
    }

    // NOTE: does this actually do anything...? it's not changing anything?
    /// negotiates if needed, returning the new session description if it exists
    pub fn negotiate_if_needed(&mut self) -> Option<SessionDescription> {
        let change = self.rtc.sdp_api();
        // change.stop_media(mid);
        if let Ok(Some(offer)) = self.signalling.negotiate_if_needed(change) {
            Some(SessionDescription(offer.to_sdp_string()))
        } else {
            None
        }
    }

    /// get permissions for this peer
    ///
    /// combines voice state and permissions
    pub fn permissions(&self) -> Permissions {
        match &self.kind {
            PeerKind::User {
                voice_state,
                permissions,
            } => Permissions {
                video: permissions.video(),
                audio: permissions.speak() && !voice_state.muted(),
                deaf: voice_state.deafened(),
            },
            PeerKind::Cascade { .. } => Permissions::all(),
        }
    }
}
