use std::sync::Arc;

use channel::ServiceThreads;
use email::ServiceEmail;
use embed::ServiceEmbed;
use media::ServiceMedia;
use messages::ServiceMessages;
use oauth2::ServiceOauth;
use permissions::ServicePermissions;
use room::ServiceRooms;
use room_analytics::ServiceRoomAnalytics;
use sessions::ServiceSessions;
use users::ServiceUsers;

use crate::{
    services::{members::ServiceMembers, presence::ServicePresence, voice::ServiceVoice},
    ServerStateInner,
};

pub mod channel;
pub mod email;
pub mod embed;
pub mod media;
pub mod members;
pub mod messages;
pub mod oauth2;
pub mod permissions;
pub mod presence;
pub mod room;
pub mod room_analytics;
pub mod sessions;
pub mod users;
pub mod voice;

pub struct Services {
    pub(super) state: Arc<ServerStateInner>,
    pub media: ServiceMedia,
    pub members: ServiceMembers,
    pub messages: ServiceMessages,
    pub perms: ServicePermissions,
    pub rooms: ServiceRooms,
    pub channels: ServiceThreads,
    pub oauth: ServiceOauth,
    pub embed: ServiceEmbed,
    pub users: ServiceUsers,
    pub sessions: ServiceSessions,
    pub email: ServiceEmail,
    pub presence: ServicePresence,
    pub voice: ServiceVoice,
    pub room_analytics: ServiceRoomAnalytics,
}

impl Services {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            embed: ServiceEmbed::new(state.clone()),
            media: ServiceMedia::new(state.clone()),
            members: ServiceMembers::new(state.clone()),
            messages: ServiceMessages::new(state.clone()),
            perms: ServicePermissions::new(state.clone()),
            rooms: ServiceRooms::new(state.clone()),
            channels: ServiceThreads::new(state.clone()),
            oauth: ServiceOauth::new(state.clone()),
            users: ServiceUsers::new(state.clone()),
            sessions: ServiceSessions::new(state.clone()),
            email: ServiceEmail::new(state.clone()),
            presence: ServicePresence::new(state.clone()),
            voice: ServiceVoice::new(state.clone()),
            room_analytics: ServiceRoomAnalytics::new(state.clone()),
            state,
        }
    }

    pub async fn start_background_tasks(&self) {
        self.channels.start_background_tasks();
        self.embed.start_workers().await;
        self.room_analytics.spawn_snapshot_task();
    }
}
