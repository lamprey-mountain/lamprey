use common::v1::types::federation::WellKnown;

use crate::prelude::*;

#[handler(routes::well_known)]
async fn well_known(
    req: Req<routes::well_known::Endpoint>,
) -> Result<routes::well_known::Response> {
    Ok(routes::well_known::Response {
        info: WellKnown {
            api_url: req.globals.config().api_url.clone(),
            cdn_url: req.globals.config().cdn_url.clone(),
        },
    })
}
