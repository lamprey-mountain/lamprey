use crate::prelude::*;

mod unversioned;
mod v1;
mod v2;

pub struct Endpoints;

impl Handlers for Endpoints {
    fn register(routes: &mut Routes) {
        routes.nest("/api", |routes| {
            v1::Endpoints::register(routes);
            v2::Endpoints::register(routes);
            unversioned::Endpoints::register(routes);
        });
    }
}
