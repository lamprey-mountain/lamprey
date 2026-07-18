use bitflags::bitflags;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::v1::types::voice::{
    TrackId,
    datachannel::{Datagram, DatagramDecodeError, EmptyDatagram, Protocol},
};

/// speaking datagram
///
/// Sent by both the client and server to indicate that someone is speaking. Clients can only send this for their own tracks.
///
/// ## binary
///
/// - 8 byte track id
/// - 1 byte flags
// TODO: tie speaking to track groups/streams
// could be fun to add other filters? like lowpass, reverb, etc (can be done client side)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeakingDatagram {
    pub track_id: TrackId,
    pub flags: SpeakingFlags,
}

bitflags! {
    /// Flags for speaking
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct SpeakingFlags: u8 {
        /// whether to send audio
        const AUDIO = 1 << 0;

        /// whether a speaking indicator should be sent
        const INDICATOR = 1 << 1;

        /// whether to use priority speaker
        const PRIORITY = 1 << 2;

        /// whether to broadcast to multiple channels
        const BROADCAST = 1 << 3;
    }
}

impl Datagram for SpeakingDatagram {
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> usize {
        buf.put_u64(self.track_id.0);
        buf.put_u8(self.flags.bits());
        9
    }

    fn decode<B: bytes::Buf>(buf: &mut B) -> Result<Self, DatagramDecodeError> {
        if buf.remaining() < 9 {
            return Err(DatagramDecodeError::UnexpectedEof);
        }

        let track_id = TrackId(buf.get_u64());
        let flags =
            SpeakingFlags::from_bits(buf.get_u8()).ok_or(DatagramDecodeError::InvalidData)?;

        Ok(SpeakingDatagram { track_id, flags })
    }
}

pub struct SpeakingProtocol;

impl Protocol for SpeakingProtocol {
    type Header = EmptyDatagram;
    type Command = SpeakingDatagram;
    type Event = SpeakingDatagram;
}
