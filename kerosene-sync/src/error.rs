// TODO: errors for connections
// enum ConnectionError {}

use lamprey::v1::types::error::SyncErrorCode;

// NOTE: maybe use Error directly?
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("{0}")]
    Api(SyncErrorCode),
    // TODO: more exact errors for sync
    // transport closed
    // internal server error (for subscriptions, initial ready state)
    // full api error
}
