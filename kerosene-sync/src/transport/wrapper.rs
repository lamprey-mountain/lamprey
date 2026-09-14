use crate::transport::{Transport, TransportSink, TransportStream};

pub struct WrapperTransport {
    sink: Box<dyn TransportSink>,
    stream: TransportStream,
}

impl WrapperTransport {
    pub fn new(sink: Box<dyn TransportSink>, stream: TransportStream) -> Self {
        Self { sink, stream }
    }
}

impl Transport for WrapperTransport {
    fn split(self: Box<Self>) -> (Box<dyn TransportSink>, TransportStream) {
        (self.sink, self.stream)
    }
}
