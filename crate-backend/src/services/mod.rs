use std::sync::Arc;

use media::ServiceMedia;
use oauth2::ServiceOauth;
use permissions::ServicePermissions;
use room::ServiceRooms;
use thread::ServiceThreads;
use url_embed::ServiceUrlEmbed;

use crate::ServerStateInner;

pub mod media;
pub mod oauth2;
pub mod permissions;
pub mod room;
pub mod thread;
pub mod url_embed;

pub struct Services {
    pub(super) state: Arc<ServerStateInner>,
    pub media: ServiceMedia,
    pub perms: ServicePermissions,
    pub rooms: ServiceRooms,
    pub threads: ServiceThreads,
    pub oauth: ServiceOauth,
    pub url_embed: ServiceUrlEmbed,
}

impl Services {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            media: ServiceMedia::new(state.clone()),
            perms: ServicePermissions::new(state.clone()),
            rooms: ServiceRooms::new(state.clone()),
            threads: ServiceThreads::new(state.clone()),
            oauth: ServiceOauth::new(state.clone()),
            url_embed: ServiceUrlEmbed::new(state.clone()),
            state,
        }
    }
}
