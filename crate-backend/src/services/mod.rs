//! services and other shared logic

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
use config::ServiceConfig;
use connections::ServiceConnections;
use email::ServiceEmail;
use embed::ServiceEmbed;
use emoji::ServiceEmoji;
use federation::ServiceFederation;
use harvest::ServiceHarvest;
use health::ServiceHealth;
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
    ServerStateInner,
    prelude::*,
    services::{
        admin::ServiceAdmin, audit_logs::ServiceAuditLogs, automod::ServiceAutomod,
        calendar::ServiceCalendar, documents::ServiceDocuments, http::ServiceHttp,
        interactions::ServiceInteractions, member_lists::ServiceMemberLists,
        notifications::ServiceNotifications, presence::ServicePresence, search::ServiceSearch,
        voice::ServiceVoice,
    },
};

pub mod admin;
pub mod audit_logs;
pub mod automod;
pub mod cache;
pub mod calendar;
pub mod channel;
pub mod config;
pub mod connections;
pub mod documents;
pub mod email;
pub mod embed;
pub mod emoji;
pub mod federation;
pub mod harvest;
pub mod health;
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
    pub config: ServiceConfig,
    pub connections: ServiceConnections,
    pub documents: ServiceDocuments,
    pub email: ServiceEmail,
    pub embed: ServiceEmbed,
    pub emoji: ServiceEmoji,
    pub federation: ServiceFederation,
    pub harvest: ServiceHarvest,
    pub health: ServiceHealth,
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
    pub users: ServiceUsers,
    pub voice: ServiceVoice,
    pub webhook: ServiceWebhooks,
    pub state: Globals,
}

impl Services {
    pub fn new(globals: Globals) -> Self {
        let state_old = Arc::new(ServerStateInner {
            tokio: tokio::runtime::Handle::current(),
            config: (*globals.config()).clone(),
            database: globals.temp_database_compat(),
            services: globals.temp_services_raw(),
            blobs: globals.blobs().clone(),
            jetstream: None,
            messaging: globals.messaging().clone(),
            globals: globals.clone(),
        });

        Self {
            admin: ServiceAdmin::new(globals.clone()),
            audit_logs: ServiceAuditLogs::new(globals.clone()),
            automod: ServiceAutomod::new(state_old.clone()),
            cache: ServiceCache::new(state_old.clone()),
            calendar: ServiceCalendar::new(state_old.clone()),
            channels: ServiceChannels::new(state_old.clone()),
            config: ServiceConfig::new(globals.clone()),
            connections: ServiceConnections::new(state_old.clone()),
            documents: ServiceDocuments::new(state_old.clone()),
            email: ServiceEmail::new(state_old.clone()),
            embed: ServiceEmbed::new(state_old.clone()),
            emoji: ServiceEmoji::new(state_old.clone()),
            federation: ServiceFederation::new(state_old.clone()),
            harvest: ServiceHarvest::new(state_old.clone()),
            health: ServiceHealth::new(globals.clone()),
            http: ServiceHttp::new(state_old.clone()),
            interactions: ServiceInteractions::new(state_old.clone()),
            ips: ServiceIps::new(state_old.clone()),
            media: ServiceMedia::new(state_old.clone()),
            member_lists: ServiceMemberLists::new(state_old.clone()),
            messages: ServiceMessages::new(state_old.clone()),
            notifications: ServiceNotifications::new(globals.clone()),
            scripts: ServiceScripts::new(state_old.clone()),
            oauth: ServiceOauth::new(state_old.clone()),
            perms: ServicePermissions::new(state_old.clone()),
            presence: ServicePresence::new(state_old.clone()),
            role: ServiceRoles::new(state_old.clone()),
            room_analytics: ServiceRoomAnalytics::new(state_old.clone()),
            rooms: ServiceRooms::new(state_old.clone()),
            room_templates: ServiceRoomTemplates::new(state_old.clone()),
            search: ServiceSearch::new(state_old.clone()),
            sessions: ServiceSessions::new(state_old.clone()),
            tag: ServiceTags::new(state_old.clone()),
            users: ServiceUsers::new(state_old.clone()),
            voice: ServiceVoice::new(state_old.clone()),
            webhook: ServiceWebhooks::new(state_old.clone()),
            state: globals,
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
        self.search.start_background_tasks();
        self.harvest.start_background_tasks();
    }

    // TODO: cleanly shutdown
    pub async fn shutdown(&self) {
        // only shut own the services that need to be shut down
        self.documents.unload_all().await;
    }
}

pub trait Service {
    fn new(state: Globals) -> Self;

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
