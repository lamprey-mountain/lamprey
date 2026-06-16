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
    #[endpoint(routes::ack_bulk_new)]
    pub async fn bulk(
        &self,
        req: Req<routes::ack_bulk_new::Endpoint>,
    ) -> Result<routes::ack_bulk_new::Response> {
        // req.auth.ensure_scopes(&[Scope::Full])?;
        // TODO

        Ok(routes::ack_bulk_new::Response {})
    }
}
