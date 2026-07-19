use std::{io::Cursor, path::Path, sync::Arc};

use symphonia::core::{
    formats::{FormatOptions, FormatReader},
    meta::MetadataOptions,
};

use crate::voice::{
    VoiceError,
    player::{
        Audio, MediaKind, Video,
        node::{AudioSource, Node, NodeHandle, NodeKind, Pause, VideoSource, Volume},
        util::{CreateSource, NodeKey},
    },
};

pub struct Player {
    format: Box<dyn FormatReader>,
    nodes: slotmap::SlotMap<NodeKey, PlayerNode>,
    edges: (),
    inner: Arc<PlayerInner>,
}

// TODO: maybe split PlayerBuilder/Player? the builder creates a static graph for player.
pub struct PlayerBuilder {
    // TODO
}

pub struct PlayerInner {
    // TODO
}

pub enum PlayerNode {
    Audio(Node<Audio, Box<dyn NodeKind>>),
    Video(Node<Video, Box<dyn NodeKind>>),
}

// TODO: add doc comments to all methods
impl Player {
    // pub fn new() -> Self {
    //     todo!()
    // }

    pub fn builder() -> Self {
        todo!()
    }
}

impl PlayerBuilder {
    pub fn create_audio_from_path(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<NodeHandle<'_, Audio, AudioSource>, VoiceError> {
        self.create_audio_inner(CreateSource::Path(path.as_ref()))
    }

    pub fn create_audio_from_buffer(
        &mut self,
        bytes: impl Into<Box<[u8]>>,
    ) -> Result<NodeHandle<'_, Audio, AudioSource>, VoiceError> {
        self.create_audio_inner(CreateSource::Memory(Cursor::new(bytes.into())))
    }

    pub fn create_video_from_path(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<NodeHandle<'_, Video, VideoSource>, VoiceError> {
        self.create_video_inner(CreateSource::Path(path.as_ref()))
    }

    pub fn create_video_from_buffer(
        &mut self,
        bytes: impl Into<Box<[u8]>>,
    ) -> Result<NodeHandle<'_, Video, VideoSource>, VoiceError> {
        self.create_video_inner(CreateSource::Memory(Cursor::new(bytes.into())))
    }

    pub fn create_av_from_path(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<
        (
            Option<NodeHandle<Audio, AudioSource>>,
            Option<NodeHandle<Video, VideoSource>>,
        ),
        VoiceError,
    > {
        self.create_av_inner(CreateSource::Path(path.as_ref()))
    }

    pub fn create_av_from_buffer(
        &mut self,
        bytes: impl Into<Box<[u8]>>,
    ) -> Result<
        (
            Option<NodeHandle<Audio, AudioSource>>,
            Option<NodeHandle<Video, VideoSource>>,
        ),
        VoiceError,
    > {
        self.create_av_inner(CreateSource::Memory(Cursor::new(bytes.into())))
    }

    fn create_audio_inner<'a>(
        &mut self,
        source: CreateSource<'a>,
    ) -> Result<NodeHandle<'_, Audio, AudioSource>, VoiceError> {
        match self.create_av_inner(source) {
            Ok((Some(a), _)) => Ok(a),
            Ok((None, _)) => todo!("return 'file has no audio' error"),
            Err(err) => Err(err),
        }
    }

    fn create_video_inner<'a>(
        &mut self,
        source: CreateSource<'a>,
    ) -> Result<Node<'_, Video, VideoSource>, VoiceError> {
        match self.create_av_inner(source) {
            Ok((_, Some(v))) => Ok(v),
            Ok((_, None)) => todo!("return 'file has no video' error"),
            Err(err) => Err(err),
        }
    }

    fn create_av_inner<'a>(
        &mut self,
        source: CreateSource<'a>,
    ) -> Result<
        (
            Option<NodeHandle<'_, Audio, AudioSource>>,
            Option<NodeHandle<'_, Video, VideoSource>>,
        ),
        VoiceError,
    > {
        let hint = source.hint();
        let mss = source.mss()?;
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();
        // TODO: don't panic
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .expect("unsupported format");
        let format = probed.format;
        // TODO: allow other formats
        // TODO: handle multiple formats in one file
        let track = format
            .tracks()
            .iter()
            // what formats are supported...?
            // .find(|t| matches!(t.codec_params.codec, symphonia::core::codecs::CODEC_TYPE_OPUS | symphonia::core::codecs::CODEC_TYPE_VORBIS))
            .find(|t| t.codec_params.codec == symphonia::core::codecs::CODEC_TYPE_OPUS)
            .expect("no supported audio tracks")
            .clone();

        // FIXME: symphonia doesnt support video

        // let decoder = symphonia::default::get_codecs()
        //     .make(&track.codec_params, &DecoderOptions::default())
        //     .unwrap();

        // Node {
        //     id: self.next_id(),
        //     kind: NodeKindAudio,
        //     _phantom: PhantomData,
        // };

        Ok(todo!())
    }

    pub fn create_pause<M: MediaKind, K>(&mut self, target: Node<'_, M, K>) -> Node<'_, M, Pause> {
        todo!()
    }

    pub fn create_volume<K>(&mut self, target: Node<'_, Audio, K>) -> Node<'_, Audio, Volume> {
        todo!()
    }

    pub fn build<K, L>(
        mut self,
        audio: Option<Node<Audio>, K>,
        video: Option<Node<Video>, L>,
    ) -> Player {
        todo!()
    }
}
