use crate::prelude::*;

pub mod ack;
// pub mod media;
pub mod message;
pub mod permission_overwrite;
pub mod unfurler;
pub mod user;

pub fn register(r: &mut Routes) {
    r.nest("/v1", |r| {
        ack::register(r);
        permission_overwrite::register(r);
        message::register(r);
        unfurler::register(r);
        user::register(r);
    });
}
