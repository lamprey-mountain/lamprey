use crate::prelude::*;

// mod backup;
// mod cross_signing;
// mod keyshare;
// mod mls;

pub struct ServiceE2EE {
    globals: Globals,
}

impl ServiceE2EE {
    pub fn new(globals: Globals) -> Self {
        Self { globals }
    }

    // TODO: handle key management
    // TODO: handle dispatching and routing e2ee sync messages
}
