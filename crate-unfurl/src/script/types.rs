use lamprey_common::v1::types::{misc::Color, EmbedType};
use rquickjs::{
    class::Trace,
    function::{FromParam, ParamRequirement, ParamsAccessor},
    Ctx, FromJs, IntoJs, JsLifetime, Value,
};
use url::Url;

use crate::{
    unfurler::EmbedGeneration,
    util::{EmbedGenerationTemplate, EmbedMediaPending},
};

#[derive(Trace, JsLifetime)]
#[rquickjs::class]
pub struct JsResponse {
    pub status: u16,
    pub url: String,
    #[qjs(skip_trace)]
    pub(crate) inner: Option<reqwest::Response>,
}

// maybe remove this? but then i get compile time errors
impl Clone for JsResponse {
    fn clone(&self) -> Self {
        todo!()
    }
}

#[rquickjs::methods]
impl JsResponse {
    #[qjs(get)]
    pub fn status(&self) -> u16 {
        self.status
    }

    #[qjs(get)]
    pub fn url(&self) -> String {
        self.url.clone()
    }

    pub async fn text(&mut self) -> rquickjs::Result<String> {
        if let Some(res) = self.inner.take() {
            res.text().await.map_err(|_| rquickjs::Error::Exception)
        } else {
            Err(rquickjs::Error::Exception) // Body already consumed
        }
    }

    // TODO: add remaining fields
}

impl<'js> FromJs<'js> for EmbedGeneration {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> rquickjs::Result<Self> {
        let obj = value.into_object().ok_or(rquickjs::Error::FromJs {
            from: "Value",
            to: "Object",
            message: Some("Expected an object".into()),
        })?;

        let title: Option<String> = obj.get("title").unwrap_or(None);
        let description: Option<String> = obj.get("description").unwrap_or(None);
        let url_str: Option<String> = obj.get("url").unwrap_or(None);
        let site_name: Option<String> = obj.get("siteName").unwrap_or(None);
        let color_str: Option<String> = obj.get("color").unwrap_or(None);
        let image_url: Option<String> = obj.get("image").unwrap_or(None);

        let url = url_str.and_then(|u| Url::parse(&u).ok());

        let mut template = EmbedGenerationTemplate {
            ty: EmbedType::Link,
            url: url.clone(),
            canonical_url: url,
            title,
            description,
            color: color_str.and_then(|c| Color::try_from_hex_string(c).ok()),
            site_name,
            // TODO: handle extra fields
            media: None,
            thumbnail: None,
            author_name: None,
            author_url: None,
            author_avatar: None,
            site_avatar: None,
        };

        if let Some(img) = image_url.and_then(|u| Url::parse(&u).ok()) {
            template.thumbnail = Some(EmbedMediaPending::new(img).into());
        }

        Ok(EmbedGeneration { embed: template })
    }
}

impl From<reqwest::Response> for JsResponse {
    fn from(value: reqwest::Response) -> Self {
        JsResponse {
            status: value.status().into(),
            url: value.url().to_string(),
            inner: Some(value),
        }
    }
}
