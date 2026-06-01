use std::time::Instant;

use bitflags::bitflags;
use bytes::Bytes;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use str0m::{format::Codec, media::MediaTime};
use uuid::Uuid;

use crate::v1::types::{voice::Mid, Channel, ChannelId, UserId};

/// a globally unique media id identifier
// TODO: use this?
pub type GlobalMid = (UserId, Mid);

/// a packet of media data
#[derive(Debug, Clone)]
pub struct MediaData {
    /// the track this this piece of media came from
    pub mid: Mid,

    /// the user this this piece of media came from
    pub user_id: UserId,

    /// the raw media data
    pub data: Bytes,

    /// the time this packet was received from the source peer
    pub network_time: Instant,

    /// the timestamp of this packet in the media stream
    pub time: MediaTime,

    /// the payload codec
    pub codec: Codec,
}

impl MediaData {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        use bytes::BufMut;

        // mid (16 bytes)
        buf.put_slice(&self.mid.0);

        // user_id (16 bytes)
        buf.put_slice(self.user_id.as_bytes());

        // codec (1 byte)
        let codec_u8 = match self.codec {
            Codec::Opus => 0,
            Codec::Vp8 => 1,
            Codec::Vp9 => 2,
            Codec::H264 => 3,
            Codec::Av1 => 4,
            // Codec::Rtx => 5, // ???
            _ => 255,
        };
        buf.put_u8(codec_u8);

        // network_time (4 bytes)
        let age = std::time::Instant::now().saturating_duration_since(self.network_time);
        buf.put_u32_le(age.as_micros() as u32);

        // time (8 bytes numer, 4 bytes freq)
        buf.put_u64_le(self.time.numer());
        buf.put_u32_le(self.time.denom());

        // data (remaining)
        buf.put_slice(&self.data);

        buf
    }

    // TODO: better errors
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        use bytes::Buf;
        let mut buf = bytes;

        if buf.remaining() < 16 + 16 + 1 + 4 + 8 + 4 {
            return Err(());
        }

        let mut mid_bytes = [0u8; 16];
        buf.copy_to_slice(&mut mid_bytes);
        let mid = Mid(mid_bytes);

        let mut user_bytes = [0u8; 16];
        buf.copy_to_slice(&mut user_bytes);
        let user_id = UserId::from(Uuid::from_bytes(user_bytes));

        let codec_u8 = buf.get_u8();
        let codec = match codec_u8 {
            0 => Codec::Opus,
            1 => Codec::Vp8,
            2 => Codec::Vp9,
            3 => Codec::H264,
            4 => Codec::Av1,
            // 5 => Codec::Rtx, // ????
            _ => Codec::Unknown,
        };

        let age_micros = buf.get_u32_le();
        let network_time =
            std::time::Instant::now() - std::time::Duration::from_micros(age_micros as u64);

        let numer = buf.get_u64_le();
        let denom = buf.get_u32_le();
        let time =
            str0m::media::MediaTime::new(numer, str0m::media::Frequency::new(denom).unwrap());

        let data = Bytes::copy_from_slice(buf.chunk());

        Ok(Self {
            mid,
            user_id,
            data,
            network_time,
            time,
            codec,
        })
    }
}

bitflags! {
    /// permissions for an sfu peer
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct SfuPermissions: u8 {
        /// whether the user has the `VoiceSpeak` permission
        const Speak = 1 << 0;

        /// whether the user has the `VoiceVideo` permission
        const Video = 1 << 1;

        /// whether the user has the `VoicePriority` permission
        const Priority = 1 << 2;
    }
}

impl SfuPermissions {
    /// whether this peer can send audio
    #[inline]
    pub fn speak(&self) -> bool {
        self.contains(Self::Speak)
    }

    /// whether this peer can send video
    #[inline]
    pub fn video(&self) -> bool {
        self.contains(Self::Video)
    }

    /// whether this peer can use priority speaker
    #[inline]
    pub fn priority(&self) -> bool {
        self.contains(Self::Priority)
    }
}

// TODO: remove
/// channel config that the sfu needs to know about
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct SfuChannel {
    pub id: ChannelId,

    // only exists for debug
    pub name: String,

    // QUESTION: does this affect video?
    pub bitrate: Option<u64>,

    // QUESTION: does this affect peers?
    // TODO: remove? this should be enforced by the api server
    pub user_limit: Option<u64>,
}

// TODO: use this instead of SfuChannel
/// configuration for a voice call
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VoiceConfig {
    /// maximum audio bitrate
    pub bitrate: u64,
    // TODO: video resolution
}

impl From<Channel> for SfuChannel {
    fn from(value: Channel) -> Self {
        Self {
            id: value.id,
            name: value.name,
            bitrate: value.bitrate,
            user_limit: value.user_limit,
        }
    }
}

/// statistics for a sfu
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct SfuStats {
    /// the number of peers connected to this sfu
    pub peer_count: u64,

    /// currently used bandwidth in bits per second
    pub bandwidth_usage: u64,

    /// maximum available bandwidth in bits per second
    pub bandwidth_max: u64,
}

// #[derive(Debug, Clone)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub enum SfuVoiceState {
//     Cascading {
//         sfu_id: SfuId,
//     },
//     Webrtc {
//         state: VoiceState,
//         permissions: SfuPermissions,
//     },
// }

// pub struct TransceiverManager {
//     map: HashMap<Mid, TransceiverConfig>,
// }

// impl TransceiverManager {
//     pub fn new() -> Self {
//         todo!()
//     }

//     /// create a new transceiver, trying to reuse one in the `inactive` or `recvonly` state first.
//     pub fn create(&mut self) -> () {
//         todo!()
//     }

//     /// upsert config for a transceiver
//     pub fn update(&mut self) -> () {
//         todo!()
//     }
// }
