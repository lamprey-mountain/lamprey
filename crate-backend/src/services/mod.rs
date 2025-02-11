use std::sync::Arc;

use media::ServiceMedia;
use permissions::ServicePermissions;
use room::ServiceRooms;
use thread::ServiceThreads;

use crate::ServerState;

pub mod media;
pub mod oauth2;
pub mod permissions;
pub mod room;
pub mod thread;

pub struct Services {
    state: Arc<ServerState>,
    pub media: ServiceMedia,
    pub perms: ServicePermissions,
    pub rooms: ServiceRooms,
    pub threads: ServiceThreads,
}

impl Services {
    pub fn new(state: Arc<ServerState>) -> Self {
        Self {
            media: ServiceMedia::new(),
            perms: ServicePermissions::new(state.clone()),
            rooms: ServiceRooms::new(state.clone()),
            threads: ServiceThreads::new(state.clone()),
            state,
        }
    }
}
