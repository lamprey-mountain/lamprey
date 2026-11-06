use crate::util::registry::export_models;

pub mod backups;
pub mod channel;
pub mod cross_signing;
pub mod keysharing;
pub mod media;
pub mod messages;
pub mod mls;

// TEMP: reexport
pub use cross_signing::{CrossSigningQuery, CrossSigningQueryRequest};
pub use keysharing::{KeyshareRespond, SessionKeyUploadRequest};
pub use messages::{Dispatch as E2EEDispatch, DispatchChannel as E2EEDispatchChannel};
pub use mls::*;

export_models!(
    use channel,
    use cross_signing,
    use keysharing,
    use media,
    use messages,
    use mls,
);
