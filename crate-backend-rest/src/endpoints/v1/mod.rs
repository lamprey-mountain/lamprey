use crate::prelude::*;

mod ack;

pub struct Endpoints;

impl Handlers for Endpoints {
    fn register(routes: &mut Routes) {
        routes.nest("/v1", |routes| {
            ack::Endpoints::register(routes);
        });
    }
}
