use crate::prelude::*;

pub mod ack;
pub mod auth;
pub mod channel;
pub mod emoji;
pub mod harvest;
// pub mod media;
pub mod message;
pub mod permission_overwrite;
pub mod relationship;
pub mod role;
pub mod room;
pub mod room_analytics;
pub mod room_member;
pub mod room_template;
pub mod search;
pub mod server;
pub mod thread;
pub mod unfurler;
pub mod user;

pub fn register(r: &mut Routes) {
    r.nest("/v1", |r| {
        ack::register(r);
        auth::register(r);
        harvest::register(r);
        permission_overwrite::register(r);
        relationship::register(r);
        channel::register(r);
        emoji::register(r);
        message::register(r);
        role::register(r);
        room::register(r);
        room_analytics::register(r);
        room_template::register(r);
        search::register(r);
        server::register(r);
        room_member::register(r);
        thread::register(r);
        unfurler::register(r);
        user::register(r);
    });
}
