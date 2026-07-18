use std::convert::Infallible;

use bitflags::bitflags;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::v1::types::voice::{
    TrackId,
    datachannel::{Datagram, DatagramSealed},
};

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

/// speaking datagram
///
/// Sent by both the client and server to indicate that someone is speaking. Clients can only send `Speaking` for their own tracks.
///
/// ## binary
///
/// - 8 byte track id
/// - 1 byte flags
// could be fun to add other filters? like lowpass, reverb, etc (can be done client side)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Speaking {
    pub track_id: TrackId,
    pub flags: SpeakingFlags,
}

impl DatagramSealed for Speaking {}

impl Datagram for Speaking {
    type DecodeError = ();
    type EncodeError = Infallible;

    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> Result<usize, Self::EncodeError> {
        buf.put_u64(self.track_id.0);
        buf.put_u8(self.flags.bits());
        Ok(9)
    }

    fn decode<B: bytes::Buf>(buf: &mut B) -> Result<Self, Self::DecodeError> {
        if buf.remaining() < 9 {
            return Err(());
        }

        let track_id = TrackId(buf.get_u64());
        let flags = SpeakingFlags::from_bits(buf.get_u8()).ok_or(())?;

        Ok(Speaking { track_id, flags })
    }
}
