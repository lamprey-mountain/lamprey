use crate::prelude::*;

mod ack;
mod media;

pub struct Endpoints;

impl Handlers for Endpoints {
    fn register(routes: &mut Routes) {
        routes.nest("/v1", |routes| {
            ack::Endpoints::register(routes);
            media::Endpoints::register(routes);
        });
    }
}
