use symphonia::core::codecs;

// TODO: seal MediaKind
pub trait MediaKind: Send + Sync + 'static {
    fn matches_codec(codec: codecs::CodecType) -> bool;
}

pub struct Audio;
pub struct Video;

impl MediaKind for Audio {
    fn matches_codec(codec: codecs::CodecType) -> bool {
        matches!(
            codec,
            codecs::CODEC_TYPE_OPUS
                | codecs::CODEC_TYPE_VORBIS
                | codecs::CODEC_TYPE_AAC
                | codecs::CODEC_TYPE_MP3
        )
    }
}

impl MediaKind for Video {
    fn matches_codec(_codec: codecs::CodecType) -> bool {
        // FIXME: symphonia doesn't support video
        false
    }
}
