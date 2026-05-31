// TODO: use a (proc)macro for this? read all files in service, create services
//
// ```rs
// // services/foo/mod.rs
// #[service]
// pub struct ServiceFoo {
//     state: Arc<ServerStateInner>,
//     ...
// }
//
// impl Service for ServiceFoo { ... }
// ```
//
// then in services/mod.rs expand out
// - pub mod foo;
// - use foo::ServiceFoo;
// - fn new() body
// - fn start_background_tasks() body
// - fn shutdown() body

use std::sync::Arc;

use cache::ServiceCache;
use channel::ServiceChannels;
use connections::ServiceConnections;
use email::ServiceEmail;
use embed::ServiceEmbed;
use emoji::ServiceEmoji;
use federation::ServiceFederation;
use ips::ServiceIps;
use media::ServiceMedia;
use messages::ServiceMessages;
use oauth2::ServiceOauth;
use permissions::ServicePermissions;
use role::ServiceRoles;
use room_analytics::ServiceRoomAnalytics;
use room_template::ServiceRoomTemplates;
use rooms::ServiceRooms;
use scripts::ServiceScripts;
use sessions::ServiceSessions;
use tag::ServiceTags;
use users::ServiceUsers;
use webhook::ServiceWebhooks;

use crate::{
    services::{
        admin::ServiceAdmin, audit_logs::ServiceAuditLogs, automod::ServiceAutomod,
        calendar::ServiceCalendar, documents::ServiceDocuments, http::ServiceHttp,
        interactions::ServiceInteractions, member_lists::ServiceMemberLists,
        notifications::ServiceNotifications, presence::ServicePresence, search::ServiceSearch,
        unread::ServiceUnread, voice::ServiceVoice,
    },
    ServerStateInner,
};

pub mod admin;
pub mod audit_logs;
pub mod automod;
pub mod cache;
pub mod calendar;
pub mod channel;
pub mod connections;
pub mod documents;
pub mod email;
pub mod embed;
pub mod emoji;
pub mod federation;
pub mod http;
pub mod interactions;
pub mod ips;
pub mod media;
pub mod member_lists;
pub mod messages;
pub mod notifications;
pub mod oauth2;
pub mod permissions;
pub mod presence;
pub mod role;
pub mod room_analytics;
pub mod room_template;
pub mod rooms;
pub mod scripts;
pub mod search;
pub mod sessions;
pub mod tag;
pub mod unread;
pub mod users;
pub mod voice;
pub mod webhook;

pub struct Services {
    pub admin: ServiceAdmin,
    pub audit_logs: ServiceAuditLogs,
    pub automod: ServiceAutomod,
    pub cache: ServiceCache,
    pub calendar: ServiceCalendar,
    pub channels: ServiceChannels,
    pub connections: ServiceConnections,
    pub documents: ServiceDocuments,
    pub email: ServiceEmail,
    pub embed: ServiceEmbed,
    pub emoji: ServiceEmoji,
    pub federation: ServiceFederation,
    pub http: ServiceHttp,
    pub interactions: ServiceInteractions,
    pub ips: ServiceIps,
    pub media: ServiceMedia,
    pub member_lists: ServiceMemberLists,
    pub messages: ServiceMessages,
    pub notifications: ServiceNotifications,
    pub oauth: ServiceOauth,
    pub perms: ServicePermissions,
    pub presence: ServicePresence,
    pub role: ServiceRoles,
    pub room_analytics: ServiceRoomAnalytics,
    pub room_templates: ServiceRoomTemplates,
    pub rooms: ServiceRooms,
    pub scripts: ServiceScripts,
    pub search: ServiceSearch,
    pub sessions: ServiceSessions,
    pub tag: ServiceTags,
    pub unread: ServiceUnread,
    pub users: ServiceUsers,
    pub voice: ServiceVoice,
    pub webhook: ServiceWebhooks,
    pub(super) state: Arc<ServerStateInner>,
}

impl Services {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            admin: ServiceAdmin::new(state.clone()),
            audit_logs: ServiceAuditLogs::new(state.clone()),
            automod: ServiceAutomod::new(state.clone()),
            cache: ServiceCache::new(state.clone()),
            calendar: ServiceCalendar::new(state.clone()),
            channels: ServiceChannels::new(state.clone()),
            connections: ServiceConnections::new(state.clone()),
            documents: ServiceDocuments::new(state.clone()),
            email: ServiceEmail::new(state.clone()),
            embed: ServiceEmbed::new(state.clone()),
            emoji: ServiceEmoji::new(state.clone()),
            federation: ServiceFederation::new(state.clone()),
            http: ServiceHttp::new(state.clone()),
            interactions: ServiceInteractions::new(state.clone()),
            ips: ServiceIps::new(state.clone()),
            media: ServiceMedia::new(state.clone()),
            member_lists: ServiceMemberLists::new(state.clone()),
            messages: ServiceMessages::new(state.clone()),
            notifications: ServiceNotifications::new(state.clone()),
            scripts: ServiceScripts::new(state.clone()),
            oauth: ServiceOauth::new(state.clone()),
            perms: ServicePermissions::new(state.clone()),
            presence: ServicePresence::new(state.clone()),
            role: ServiceRoles::new(state.clone()),
            room_analytics: ServiceRoomAnalytics::new(state.clone()),
            rooms: ServiceRooms::new(state.clone()),
            room_templates: ServiceRoomTemplates::new(state.clone()),
            search: ServiceSearch::new(state.clone()),
            sessions: ServiceSessions::new(state.clone()),
            tag: ServiceTags::new(state.clone()),
            unread: ServiceUnread::new(state.clone()),
            users: ServiceUsers::new(state.clone()),
            voice: ServiceVoice::new(state.clone()),
            webhook: ServiceWebhooks::new(state.clone()),
            state,
        }
    }

    pub async fn start_background_tasks(&self) {
        self.email.start_background_tasks();
        self.admin.start_background_tasks();
        self.channels.start_background_tasks();
        self.documents.start_background_tasks();
        self.federation.start_background_tasks();
        self.notifications.start_background_tasks();
        self.embed.start_workers().await;
        self.room_analytics.spawn_snapshot_task();
        self.cache.start_background_tasks();
        self.member_lists.start_background_tasks();
        self.media.start_background_tasks();
        self.search.start_background_tasks();
    }

    // TODO: cleanly shutdown
    pub async fn shutdown(&self) {
        // only shut own the services that need to be shut down
        self.documents.unload_all().await;
    }
}

pub trait Service {
    fn new(state: Arc<ServerStateInner>) -> Self;

    /// start background tasks
    fn start_background_tasks(&self) {}

    // /// cleanly shutdown
    // fn shutdown(&self) -> impl Future {}
}

// pub trait ResourceService {
//     type Id;
//     type Item;

//     async fn get(&self, id: Self::Id) -> Result<Option<Self::Item>>;
// }

// pub trait CreatableResourceService: ResourceService {
//     type Create;

//     fn create(&self, create: Self::Create) -> Result<Self::Item>;
// }

// pub trait UpdateableResourceService: ResourceService {}
// pub trait DeleteableResourceService: ResourceService {}
// pub trait ListableResourceService: ResourceService {}
