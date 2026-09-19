use common::v1::types::federation::WellKnown;

use crate::prelude::*;

#[handler(routes::well_known)]
pub async fn well_known(
    req: Req<routes::well_known::Endpoint>,
) -> Result<routes::well_known::Response> {
    let globals = req.globals();
    let config = globals.config();

    Ok(routes::well_known::Response {
        info: WellKnown {
            api_url: config.api_url.clone(),
            cdn_url: config.cdn_url.clone(),
        },
    })
}

export_routes!(well_known);
