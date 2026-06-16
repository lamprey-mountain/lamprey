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
    #[endpoint(routes::media_create_new)]
    pub async fn create(
        &self,
        req: Req<routes::media_create_new::Endpoint>,
    ) -> Result<routes::media_create_new::Response> {
        // req.auth.ensure_scopes(&[Scope::Full])?;
        // TODO

        Ok(routes::media_create_new::Response { media: todo!() })
    }

    #[endpoint(routes::media_upload)]
    pub async fn upload(
        &self,
        req: Req<routes::media_upload::Endpoint>,
    ) -> Result<routes::media_upload::Response> {
        // req.auth.ensure_scopes(&[Scope::Full])?;
        // TODO

        Ok(routes::media_upload::Response {
            upload_offset: todo!(),
            content_length: todo!(),
        })
    }
}
