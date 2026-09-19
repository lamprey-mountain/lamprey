use crate::prelude::*;

pub mod unversioned;
pub mod v1;
// pub mod v2;

pub fn register(r: &mut crate::Routes) {
    unversioned::register(r);
    r.nest("/api", |r| {
        v1::register(r);
    });
}
