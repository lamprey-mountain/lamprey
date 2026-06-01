//! Cascading peer — forwards media and speaking data to/from a remote SFU via the QUIC backbone.

use common::{
    v1::types::{
        voice::{
            internal::MediaData,
            messages::{BackboneDatagram, BackboneDispatch, BackboneDispatchEnvelope},
            KeyframeRequestKind, Mid, Rid, SpeakingWithUserId, TrackMetadataWithUserId,
        },
        ChannelId, SfuId,
    },
    v2::types::UserId,
};
use tracing::warn;

use crate::backbone::BackboneComms;

// TODO: maybe use another str0m Rtc for webrtc instead of quic datagrams?

/// a cascading peer that bridges this shard to a remote SFU
pub struct Cascade {
    /// the remote SFU this cascading peer represents
    pub remote_sfu: SfuId,

    /// the channel this cascade is for
    pub channel_id: ChannelId,

    /// backbone handle for sending data
    backbone: BackboneComms,
}

impl Cascade {
    pub fn new(remote_sfu: SfuId, channel_id: ChannelId, backbone: BackboneComms) -> Self {
        Self {
            remote_sfu,
            channel_id,
            backbone,
        }
    }

    /// forward media data to the remote SFU via unreliable datagram
    pub fn forward_media(&self, media: MediaData) {
        self.backbone
            .broadcast_datagram(&[self.remote_sfu], BackboneDatagram::Media(media));
    }

    /// forward speaking indicator to the remote SFU via unreliable datagram
    pub fn forward_speaking(&self, speaking: SpeakingWithUserId) {
        self.backbone
            .broadcast_datagram(&[self.remote_sfu], BackboneDatagram::Speaking(speaking));
    }

    /// forward a keyframe request to the remote SFU via reliable dispatch
    pub fn forward_keyframe(
        &self,
        mid: Mid,
        rid: Option<Rid>,
        kind: KeyframeRequestKind,
        user_id: UserId,
    ) {
        let dispatch = BackboneDispatchEnvelope {
            nonce: None,
            dispatch: BackboneDispatch::Keyframe {
                user_id,
                mid,
                rid,
                kind,
            },
        };

        if let Err(e) = self.backbone.send_dispatch(self.remote_sfu, dispatch) {
            warn!(
                "failed to send keyframe dispatch to remote SFU {}: {:?}",
                self.remote_sfu, e
            );
        }
    }

    /// notify the remote SFU that new tracks have been created
    pub fn forward_track_create(&self, tracks: Vec<TrackMetadataWithUserId>) {
        let dispatch = BackboneDispatchEnvelope {
            nonce: None,
            dispatch: BackboneDispatch::TrackCreate {
                channel_id: self.channel_id,
                tracks,
            },
        };

        if let Err(e) = self.backbone.send_dispatch(self.remote_sfu, dispatch) {
            warn!(
                "failed to send track create dispatch to remote SFU {}: {:?}",
                self.remote_sfu, e
            );
        }
    }

    /// notify the remote SFU that tracks have been removed
    pub fn forward_track_remove(&self, tracks: Vec<(Mid, UserId)>) {
        let dispatch = BackboneDispatchEnvelope {
            nonce: None,
            dispatch: BackboneDispatch::TrackRemove {
                channel_id: self.channel_id,
                tracks,
            },
        };

        if let Err(e) = self.backbone.send_dispatch(self.remote_sfu, dispatch) {
            warn!(
                "failed to send track remove dispatch to remote SFU {}: {:?}",
                self.remote_sfu, e
            );
        }
    }
}
