use crate::prelude::*;

mod v1;

// TODO: add
// mod v2;
// mod unversioned; (for well_known)

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
