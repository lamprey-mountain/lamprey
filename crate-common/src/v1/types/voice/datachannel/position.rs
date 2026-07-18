use crate::v1::types::voice::{
    TrackId,
    datachannel::{Datagram, DatagramDecodeError, EmptyDatagram, Protocol},
};
use bytes::{Buf, BufMut};

/// an audio position datagram
///
/// Sent by both the client and server to indicate that a media's position changed. Clients can only send this for their own tracks.
///
/// ## binary
///
/// - 8 byte track id
/// - 1 byte tag (0 = Position, 1 = Direction)
/// - 12 bytes coordinates (x: f32, y: f32, z: f32)
// TODO: tie position to track groups/streams
#[derive(Debug, Clone)]
pub struct PositionDatagram {
    pub track_id: TrackId,
    pub update: PositionDatagramUpdate,
}

#[derive(Debug, Clone)]
pub enum PositionDatagramUpdate {
    /// set the position of an audio source
    Position { x: f32, y: f32, z: f32 },

    /// set where an audio source is pointing
    Direction { x: f32, y: f32, z: f32 },
}

impl Datagram for PositionDatagram {
    fn encode<B: BufMut>(&self, buf: &mut B) -> usize {
        buf.put_u64(self.track_id.0);
        match self.update {
            PositionDatagramUpdate::Position { x, y, z } => {
                buf.put_u8(0);
                buf.put_f32(x);
                buf.put_f32(y);
                buf.put_f32(z);
            }
            PositionDatagramUpdate::Direction { x, y, z } => {
                buf.put_u8(1);
                buf.put_f32(x);
                buf.put_f32(y);
                buf.put_f32(z);
            }
        }
        21
    }

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, DatagramDecodeError> {
        if buf.remaining() < 21 {
            return Err(DatagramDecodeError::UnexpectedEof);
        }
        let track_id = TrackId(buf.get_u64());
        let tag = buf.get_u8();
        let x = buf.get_f32();
        let y = buf.get_f32();
        let z = buf.get_f32();

        let update = match tag {
            0 => PositionDatagramUpdate::Position { x, y, z },
            1 => PositionDatagramUpdate::Direction { x, y, z },
            _ => return Err(DatagramDecodeError::InvalidData),
        };

        Ok(PositionDatagram { track_id, update })
    }
}

pub struct PositionProtocol;

impl Protocol for PositionProtocol {
    type Header = EmptyDatagram;
    type Command = PositionDatagram;
    type Event = PositionDatagram;
}
