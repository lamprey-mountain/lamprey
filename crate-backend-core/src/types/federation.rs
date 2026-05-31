use common::v1::types::federation::RemoteEpoch;

// TODO: save (cache?) in db?
#[derive(Debug, Clone)]
pub struct ServerData {
    /// if we're connected and syncing
    pub sync: Option<ServerSync>,
    // pub sync_incoming: Option<ServerSync>,
    // pub sync_outgoing: Option<ServerSync>,
}

#[derive(Debug, Clone)]
pub struct ServerSync {
    pub state: ServerSyncState,
    pub epoch: RemoteEpoch,
}

#[derive(Debug, Clone)]
pub enum ServerSyncState {
    /// currently pushing events to this remote server
    Active,

    /// server is lagging behind
    Lagged,

    /// could not connect to the server
    Disconnected,
}
