use std::{io, path::Path};

use crate::voice::VoiceTrackOutgoing;

// copied from symphonia
trait MediaSource: io::Read + io::Seek + Send + Sync {
    fn is_seekable(&self) -> bool;

    fn byte_len(&self) -> Option<u64>;
}

/// stream media to a rtc track
struct MediaPlayer_;

trait MediaPlayer {
    fn new(track: VoiceTrackOutgoing) -> Self;

    /// play an audio source. replaces the existing audio source, if any.
    fn play(&self, source: &dyn MediaSource);

    /// pause or unpause this player
    fn pause(&self, paused: bool);
}

struct AudioSource_;

trait AudioSource {
    /// create a new audio source that plays a file
    fn new_from_path(path: impl AsRef<Path>) -> Self;
    fn new_from_bytes(bytes: impl AsRef<[u8]>) -> Self;
    // fn new_from_stream(stream: impl Stream<[u8]>) -> Self;

    // fn set_volume() -> Self;

    // TODO: some way to set volume while its playing
}
