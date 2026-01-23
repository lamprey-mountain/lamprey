use std::sync::Arc;

use cache::ServiceCache;
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
    services::{
        admin::ServiceAdmin, automod::ServiceAutomod, documents::ServiceDocuments,
        http::ServiceHttp, members::ServiceMembers, notifications::ServiceNotifications,
        presence::ServicePresence, search::ServiceSearch, unread::ServiceUnread,
        voice::ServiceVoice,
    },
    ServerStateInner,
};

pub mod admin;
pub mod automod;
pub mod cache;
pub mod channel;
pub mod documents;
pub mod email;
pub mod embed;
pub mod http;
pub mod media;
pub mod members;
pub mod member_lists;
pub mod messages;
pub mod notifications;
pub mod oauth2;
pub mod permissions;
pub mod presence;
pub mod room;
pub mod room_analytics;
pub mod search;
pub mod sessions;
pub mod unread;
pub mod users;
pub mod voice;

pub struct Services {
    pub admin: ServiceAdmin,
    pub automod: ServiceAutomod,
    pub cache: ServiceCache,
    pub channels: ServiceThreads,
    pub documents: ServiceDocuments,
    pub email: ServiceEmail,
    pub embed: ServiceEmbed,
    pub http: ServiceHttp,
    pub media: ServiceMedia,
    pub members: ServiceMembers,
    pub messages: ServiceMessages,
    pub notifications: ServiceNotifications,
    pub oauth: ServiceOauth,
    pub perms: ServicePermissions,
    pub presence: ServicePresence,
    pub room_analytics: ServiceRoomAnalytics,
    pub rooms: ServiceRooms,
    pub search: ServiceSearch,
    pub sessions: ServiceSessions,
    pub unread: ServiceUnread,
    pub users: ServiceUsers,
    pub voice: ServiceVoice,
    pub(super) state: Arc<ServerStateInner>,
}

impl Services {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            admin: ServiceAdmin::new(state.clone()),
            automod: ServiceAutomod::new(state.clone()),
            cache: ServiceCache::new(state.clone()),
            channels: ServiceThreads::new(state.clone()),
            documents: ServiceDocuments::new(state.clone()),
            email: ServiceEmail::new(state.clone()),
            embed: ServiceEmbed::new(state.clone()),
            http: ServiceHttp::new(state.clone()),
            media: ServiceMedia::new(state.clone()),
            members: ServiceMembers::new(state.clone()),
            messages: ServiceMessages::new(state.clone()),
            notifications: ServiceNotifications::new(state.clone()),
            oauth: ServiceOauth::new(state.clone()),
            perms: ServicePermissions::new(state.clone()),
            presence: ServicePresence::new(state.clone()),
            room_analytics: ServiceRoomAnalytics::new(state.clone()),
            rooms: ServiceRooms::new(state.clone()),
            search: ServiceSearch::new(state.clone()),
            sessions: ServiceSessions::new(state.clone()),
            unread: ServiceUnread::new(state.clone()),
            users: ServiceUsers::new(state.clone()),
            voice: ServiceVoice::new(state.clone()),
            state,
        }
    }

    pub async fn start_background_tasks(&self) {
        self.channels.start_background_tasks();
        self.documents.start_background_tasks();
        self.embed.start_workers().await;
        self.room_analytics.spawn_snapshot_task();

        let state = self.state.clone();
        tokio::spawn(async move {
            let mut rx = state.sushi.subscribe();
            while let Ok((msg, _)) = rx.recv().await {
                state.services().cache.handle_sync(&msg).await;
            }
        });
    }
}
