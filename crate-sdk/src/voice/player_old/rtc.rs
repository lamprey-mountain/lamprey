//! types relevant for rtc

use std::marker::PhantomData;

use str0m::media::MediaTime;

use crate::voice::player::util::MediaKind;

#[derive(Debug)]
pub struct Packet<K: MediaKind> {
    pub data: Box<[u8]>,
    pub time: MediaTime,
    pub(super) _kind: PhantomData<K>,
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
