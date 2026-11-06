//! Sync protocol

pub mod dispatch;
pub mod stream;
pub mod transport;
pub mod visibility;

use crate::util::registry::export_models;

export_models!(use transport, use stream);

// TODO: define errors here?
// eg. "too many subscriptions/open streams"
// errors are probably somewhat transport specific though
