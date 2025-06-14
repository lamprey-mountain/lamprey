use std::sync::Arc;

use email::ServiceEmail;
use embed::ServiceEmbed;
use media::ServiceMedia;
use messages::ServiceMessages;
use oauth2::ServiceOauth;
use permissions::ServicePermissions;
use room::ServiceRooms;
use sessions::ServiceSessions;
use thread::ServiceThreads;
use users::ServiceUsers;

use crate::ServerStateInner;

pub mod email;
pub mod embed;
pub mod media;
pub mod messages;
pub mod oauth2;
pub mod permissions;
pub mod room;
pub mod sessions;
pub mod thread;
pub mod users;

pub struct Services {
    pub(super) state: Arc<ServerStateInner>,
    pub media: ServiceMedia,
    pub messages: ServiceMessages,
    pub perms: ServicePermissions,
    pub rooms: ServiceRooms,
    pub threads: ServiceThreads,
    pub oauth: ServiceOauth,
    pub embed: ServiceEmbed,
    pub users: ServiceUsers,
    pub sessions: ServiceSessions,
    pub email: ServiceEmail,
}

impl Services {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            embed: ServiceEmbed::new(state.clone()),
            media: ServiceMedia::new(state.clone()),
            messages: ServiceMessages::new(state.clone()),
            perms: ServicePermissions::new(state.clone()),
            rooms: ServiceRooms::new(state.clone()),
            threads: ServiceThreads::new(state.clone()),
            oauth: ServiceOauth::new(state.clone()),
            users: ServiceUsers::new(state.clone()),
            sessions: ServiceSessions::new(state.clone()),
            email: ServiceEmail::new(state.clone()),
            state,
        }
    }
}
