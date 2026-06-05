use common::v1::types::federation::FederationEpoch;

// TODO: save (cache?) in db?
#[derive(Debug, Clone)]
pub struct ServerData {
    /// sync state for remote -> local
    pub sync_incoming: Option<ServerSync>,

    /// sync state for local -> remote
    pub sync_outgoing: Option<ServerSync>,
}

#[derive(Debug, Clone)]
pub struct ServerSync {
    // pub state: ServerSyncState,
    pub epoch: FederationEpoch,
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
