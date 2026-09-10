use crate::prelude::*;

// mod unversioned;
mod v1;
// mod v2;

pub fn register(r: &mut crate::Routes) {
    r.nest("/api", |r| {
        v1::register(r);
    });
}
