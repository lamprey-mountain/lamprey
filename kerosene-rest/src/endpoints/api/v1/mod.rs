use crate::prelude::*;

pub mod ack;
// pub mod media;

pub fn register(r: &mut Routes) {
    r.nest("/v1", |r| {
        ack::register(r);
    });
}
