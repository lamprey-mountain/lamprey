use std::{io::Cursor, marker::PhantomData, path::Path};

use str0m::media::MediaTime;
use symphonia::core::{io::MediaSourceStream, probe::Hint};

use crate::voice::VoiceError;

// TODO: seal MediaKind
pub trait MediaKind: Send + Sync + 'static {}
pub struct Audio;
pub struct Video;
impl MediaKind for Audio {}
impl MediaKind for Video {}

#[derive(Debug)]
pub struct Packet<K: MediaKind> {
    pub data: Box<[u8]>,
    pub time: MediaTime,
    _kind: PhantomData<K>,
}

impl<M: MediaKind> Packet<M> {
    pub fn empty(time: MediaTime) -> Self {
        Packet {
            data: Box::default(),
            time,
            _kind: PhantomData,
        }
    }
}

pub enum CreateSource<'a> {
    Path(&'a Path),
    Memory(Cursor<Box<[u8]>>),
}

impl<'a> CreateSource<'a> {
    pub fn hint(&self) -> Hint {
        let mut hint = Hint::new();
        if let Self::Path(path) = self {
            if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                hint.with_extension(ext);
            }
        }
        hint
    }

    pub fn mss(self) -> Result<MediaSourceStream, VoiceError> {
        match self {
            Self::Path(path) => {
                let file = std::fs::File::open(path)?;
                Ok(MediaSourceStream::new(Box::new(file), Default::default()))
            }
            Self::Memory(cursor) => {
                Ok(MediaSourceStream::new(Box::new(cursor), Default::default()))
            }
        }
    }
}

// TODO: impl From<Path> for CreateSource
// TODO: impl From<Box[u8]> for CreateSource
// TODO: impl From<Vec<u8>> for CreateSource

slotmap::new_key_type! {
    pub struct NodeKey;
}
