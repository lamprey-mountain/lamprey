use std::marker::PhantomData;

use crate::prelude::*;
use async_trait::async_trait;
use wtransport::{RecvStream, SendStream};
use lamprey::v1::types::SyncFormat;

use super::{TransportSinkV2, TransportStreamV2, TransportV2};

pub struct WebtransportTransportV2<C, E> {
    send: SendStream,
    recv: RecvStream,
    format: SyncFormat,
    is_compressed: bool,
    _marker: PhantomData<(C, E)>,
}

impl<C, E> WebtransportTransportV2<C, E> {
    pub fn new(send: SendStream, recv: RecvStream, format: SyncFormat, is_compressed: bool) -> Self {
        Self {
            send,
            recv,
            format,
            is_compressed,
            _marker: PhantomData,
        }
    }
}

// TODO: implement TransportV2 for WebtransportTransportV2

