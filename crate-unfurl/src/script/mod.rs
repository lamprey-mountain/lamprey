use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Response;
use rquickjs::{
    AsyncContext, AsyncRuntime, FromJs, Function, JsLifetime, Module, Persistent, Value,
};
use url::Url;

use crate::script::types::JsResponse;
use crate::{error::UnfurlError, unfurler::EmbedGeneration};

use crate::UnfurlPlugin;

mod html;
mod types;

type Error = UnfurlError;

/// manages multiple script plugins
pub struct ScriptRuntime {
    rt: AsyncRuntime,
    ctx_template: AsyncContext,
}

/// implement plugins with javascript
pub struct ScriptPlugin {
    pub name: String,
    ctx: AsyncContext,
    inner: Arc<ScriptPluginInner>,
}

/// inner struct to hold non-Send/Sync js handles
struct ScriptPluginInner {
    process_url_fn: Option<Persistent<Function<'static>>>,
    accepts_response_fn: Option<Persistent<Function<'static>>>,
    process_response_fn: Option<Persistent<Function<'static>>>,
}

impl ScriptRuntime {
    pub async fn init(memory_limit: Option<usize>) -> Result<Self, Error> {
        let rt = rquickjs::AsyncRuntime::new()?;

        if let Some(memory_limit) = memory_limit {
            rt.set_memory_limit(memory_limit).await;
        }

        // rt.set_loader(resolver, loader);

        let ctx = rquickjs::AsyncContext::full(&rt).await?;
        ctx.with(|ctx| {
            let globals = ctx.globals();

            html::register(ctx.clone())?;

            // globals.set("logger", todo!())?;

            Ok::<_, Error>(())
        })
        .await?;

        Ok(Self {
            rt,
            ctx_template: ctx,
        })
    }

    pub async fn load(&self, script_name: &str, source: &str) -> Result<ScriptPlugin, Error> {
        let ctx = self.ctx_template.clone();

        let (plugin_name, inner) = ctx
            .with(|ctx| {
                let module = Module::declare(ctx.clone(), script_name, source)?;
                let (module, _) = module.eval()?;

                let plugin_name: String = module.get("name")?;
                let process_url: Option<Function> = module.get("process_url")?;
                let accepts_response: Option<Function> = module.get("accepts_response")?;
                let process_response: Option<Function> = module.get("process_response")?;

                let inner = ScriptPluginInner {
                    process_url_fn: process_url.map(|f| Persistent::save(&ctx, f)),
                    accepts_response_fn: accepts_response.map(|f| Persistent::save(&ctx, f)),
                    process_response_fn: process_response.map(|f| Persistent::save(&ctx, f)),
                };

                Ok::<(String, ScriptPluginInner), Error>((plugin_name, inner))
            })
            .await?;

        Ok(ScriptPlugin {
            name: plugin_name,
            ctx,
            inner: Arc::new(inner),
        })
    }
}

#[async_trait]
impl UnfurlPlugin for ScriptPlugin {
    fn name(&self) -> &'static str {
        "ScriptPlugin"
    }

    async fn process_url(&self, url: &Url) -> Result<Option<Vec<EmbedGeneration>>, UnfurlError> {
        let inner = self.inner.clone();
        let Some(ref func) = inner.process_url_fn else {
            return Ok(None);
        };

        let url_str = url.to_string();
        let func_persistent = func.clone();

        let res = self
            .ctx
            .with(|ctx| {
                let func = func_persistent.restore(&ctx)?;
                let result: Value = func.call((url_str,))?;

                if result.is_null() || result.is_undefined() {
                    Ok(None)
                } else {
                    let embeds: Vec<EmbedGeneration> = FromJs::from_js(&ctx, result)?;
                    Ok(Some(embeds))
                }
            })
            .await?;

        Ok(res)
    }

    fn accepts_response(&self, _res: &Response) -> bool {
        todo!()
    }

    async fn process_response(
        &self,
        url: &Url,
        res: Response,
    ) -> Result<Vec<EmbedGeneration>, UnfurlError> {
        let inner = self.inner.clone();
        let Some(ref func) = inner.process_response_fn else {
            return Err(UnfurlError::MissingImplementation);
        };

        let url_str = url.to_string();
        let js_res = JsResponse::from(res);
        let func_persistent = func.clone();

        let res = self
            .ctx
            .with(|ctx| {
                let func = func_persistent.restore(&ctx)?;
                let result: Value = func.call((url_str, js_res))?;
                let embeds: Vec<EmbedGeneration> = FromJs::from_js(&ctx, result)?;
                Ok::<_, UnfurlError>(embeds)
            })
            .await?;

        Ok(res)
    }
}
