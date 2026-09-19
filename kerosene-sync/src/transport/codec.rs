// TODO: impl and use this

// see https://docs.rs/tokio-util/latest/tokio_util/codec/index.html

// NOTE: maybe i should use this instead of compress?
pub trait Codec {
    type Input;
    type Output;

    // TODO: finish out trait
}

// NOTE: do i want a single struct Json; that handles both send/recv? or split into two?

pub struct JsonCommands {
    _nope: (),
}

pub struct JsonEvents {
    _nope: (),
}

pub struct Msgpack {
    _nope: (),
}

impl Codec for Json {
    type Input = &[u8];
    type Output = serde_json::Value;
}

// pub trait TransportExt: Transport {
//     fn framed<C: codec>(self, c: Codec) -> C::Transport {
//         todo!()
//     }
// }
