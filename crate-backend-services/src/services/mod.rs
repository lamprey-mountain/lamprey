pub mod channels;
pub mod documents;
pub mod member_lists;
pub mod notifications;
// pub mod permissions;
pub mod preferences;
pub mod rooms;
pub mod users;
pub mod federation;
pub mod http;
pub mod oauth2;

pub struct Services {
    pub channels: channels::Service,
    pub documents: documents::Service,
    // pub config: config::Service,
    // pub documents: documents::Service,
    // pub email: email::Service,
    // pub embed: embed::Service,
    pub federation: federation::Service,
    pub http: http::Service,
    pub member_lists: member_lists::Service,
    pub notifications: notifications::Service,
    // pub permissions: permissions::Service,
    pub preferences: preferences::Service,
    pub rooms: rooms::Service,
    // pub search: search::Service,
    // pub voice: voice::Service,
    pub users: users::Service,
    // TODO: copy services from crate-backend/src/services/mod.rs
}

impl Services {
    pub fn new(/* ... */) -> Self {
        todo!()
    }
}
