use std::sync::Arc;

use media::ServiceMedia;
use oauth2::ServiceOauth;
use permissions::ServicePermissions;
use room::ServiceRooms;
use thread::ServiceThreads;

use crate::ServerStateInner;

pub mod media;
pub mod oauth2;
pub mod permissions;
pub mod room;
pub mod thread;

pub struct Services {
    pub(super) state: Arc<ServerStateInner>,
    pub media: ServiceMedia,
    pub perms: ServicePermissions,
    pub rooms: ServiceRooms,
    pub threads: ServiceThreads,
    pub oauth: ServiceOauth,
}

impl Services {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            media: ServiceMedia::new(),
            perms: ServicePermissions::new(state.clone()),
            rooms: ServiceRooms::new(state.clone()),
            threads: ServiceThreads::new(state.clone()),
            oauth: ServiceOauth::new(state.clone()),
            state,
        }
    }
}
