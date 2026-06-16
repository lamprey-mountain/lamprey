use crate::prelude::*;

mod v1;
mod v2;
mod unversioned;

pub struct Endpoints;

impl Handlers for Endpoints {
    fn register(routes: &mut Routes) {
        routes.nest("/api", |routes| {
            v1::Endpoints::register(routes);
            // etc
            // v2::Endpoints::register(routes);
            // unversioned::Endpoints::register(routes);
        });
    }
}
