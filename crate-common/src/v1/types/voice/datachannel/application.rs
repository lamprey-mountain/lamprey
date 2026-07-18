use crate::v1::types::{ApplicationId, UserId, voice::datachannel::Protocol};

#[derive(Debug, Clone)]
pub struct ApplicationBroadcastHeader {
    /// the id of the application which is opening this channel
    pub application_id: ApplicationId,

    /// the name, for pubsub
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ApplicationDataDatagram {
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ApplicationConnectHeader {
    /// the id of the application which is opening this channel
    pub application_id: ApplicationId,

    /// the id of the user to connect to
    pub user_id: UserId,

    /// the connection id of the user to connect to, if the user has multiple connections
    pub connection_id: Option<ConnectionId>,

    /// human readable name for this connection
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ApplicationConnectedHeader {
    /// the id of the application which opened this channel
    pub application_id: ApplicationId,
    pub name: String,
}

// TODO: impl Datagram

pub struct ApplicationBroadcastProtocol;
pub struct ApplicationConnectProtocol;
pub struct ApplicationConnectedProtocol;

impl Protocol for ApplicationBroadcastProtocol {
    type Header = ApplicationBroadcastHeader;
    type Command = ApplicationDataDatagram;
    type Event = ApplicationDataDatagram;
}

impl Protocol for ApplicationConnectProtocol {
    type Header = ApplicationConnectHeader;
    type Command = ApplicationDataDatagram;
    type Event = ApplicationDataDatagram;
}

impl Protocol for ApplicationConnectedProtocol {
    type Header = ApplicationConnectedHeader;
    type Command = ApplicationDataDatagram;
    type Event = ApplicationDataDatagram;
}
