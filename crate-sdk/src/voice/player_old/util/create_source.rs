use std::{io::Cursor, path::Path};

use symphonia::core::{io::MediaSourceStream, probe::Hint};

use crate::voice::VoiceError;

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
