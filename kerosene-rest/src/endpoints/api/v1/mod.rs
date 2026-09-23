use crate::prelude::*;

pub mod ack;
// pub mod media;
pub mod message;
pub mod user;

pub fn register(r: &mut Routes) {
    r.nest("/v1", |r| {
        ack::register(r);
        message::register(r);
        user::register(r);
    });
}
