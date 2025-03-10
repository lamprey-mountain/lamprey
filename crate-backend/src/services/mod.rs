use std::sync::Arc;

use media::ServiceMedia;
use oauth2::ServiceOauth;
use permissions::ServicePermissions;
use room::ServiceRooms;
use sessions::ServiceSessions;
use thread::ServiceThreads;
use url_embed::ServiceUrlEmbed;
use users::ServiceUsers;

use crate::ServerStateInner;

pub mod media;
pub mod oauth2;
pub mod permissions;
pub mod room;
pub mod sessions;
pub mod thread;
pub mod url_embed;
pub mod users;

pub struct Services {
    pub(super) state: Arc<ServerStateInner>,
    pub media: ServiceMedia,
    pub perms: ServicePermissions,
    pub rooms: ServiceRooms,
    pub threads: ServiceThreads,
    pub oauth: ServiceOauth,
    pub url_embed: ServiceUrlEmbed,
    pub users: ServiceUsers,
    pub sessions: ServiceSessions,
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
            users: ServiceUsers::new(state.clone()),
            sessions: ServiceSessions::new(state.clone()),
            state,
        }
    }
}
