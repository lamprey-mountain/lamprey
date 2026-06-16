use common::v1::types::federation::WellKnown;

use crate::prelude::*;

pub struct Endpoints {
    globals: Globals,
}

impl Endpoints {
    pub fn new(globals: Globals) -> Self {
        Self { globals }
    }
}

#[handlers]
impl Endpoints {
    #[endpoint(routes::well_known::well_known)]
    pub async fn well_known(
        &self,
        _req: Req<routes::well_known::well_known::Endpoint>,
    ) -> Result<routes::well_known::well_known::Response> {
        Ok(routes::well_known::well_known::Response {
            info: WellKnown {
                api_url: self.globals.config().api_url.clone(),
                cdn_url: self.globals.config().cdn_url.clone(),
            },
        })
    }
}
