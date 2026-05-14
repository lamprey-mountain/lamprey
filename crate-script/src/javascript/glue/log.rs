use std::{collections::HashMap, sync::Arc};

use common::v1::types::{
    metadata::Metadata,
    redex::{EvalLogEntry, EvalLogLevel, EvalLogSource},
    util::Time,
    RedexId,
};
use rquickjs::{
    class::{Trace, Tracer},
    function::{FromParam, ParamRequirement},
    Ctx, FromJs, JsLifetime,
};
use tokio::sync::broadcast::Sender;
use validator::Validate;

use crate::engine::ExecutionEvent;

/// logging utilities exposed to scripts
#[rquickjs::class]
#[derive(Clone, JsLifetime)]
pub struct Logger {
    sender: Sender<Arc<ExecutionEvent>>,
    script_id: RedexId,
}

// none of these fields need to be traced
impl<'js> Trace<'js> for Logger {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

impl Logger {
    pub(crate) fn new(sender: Sender<Arc<ExecutionEvent>>, script_id: RedexId) -> Self {
        Self { sender, script_id }
    }
}

struct LoggerParams {
    content: String,
    attrs: Metadata,
}

impl<'js> FromParam<'js> for LoggerParams {
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::single().combine(ParamRequirement::optional())
    }

    fn from_param<'a>(
        params: &mut rquickjs::function::ParamsAccessor<'a, 'js>,
    ) -> rquickjs::Result<Self> {
        let content = params
            .arg()
            .into_string()
            .ok_or(rquickjs::Error::new_from_js("your data", "string"))?
            .to_string()?;

        let attrs: HashMap<String, String> = if params.is_empty() {
            HashMap::new()
        } else {
            match params.arg() {
                m if m.is_null() || m.is_undefined() => HashMap::new(),
                m => HashMap::from_js(params.ctx(), m)?,
            }
        };

        let attrs = Metadata(attrs);
        attrs.validate().map_err(|err| {
            rquickjs::Error::new_from_js_message("object", "MessageMetadata", err.to_string())
        })?;

        Ok(Self { content, attrs })
    }
}

impl Logger {
    fn log_impl<'js>(
        &self,
        _ctx: Ctx<'js>, // TODO: remove
        level: EvalLogLevel,
        params: LoggerParams,
    ) -> rquickjs::Result<()> {
        // TODO: get source span/line/column for logging
        // this kinda works: dbg!(Exception::from_message(ctx.clone(), "testing"));
        // but i'd have to manually parse the stack trace

        let entry = EvalLogEntry {
            id: 0,
            created_at: Time::now_utc(),
            level,
            source: EvalLogSource::Redex {
                redex_id: self.script_id,
                trace_id: None,
                target: None,
                line: None,
                column: None,
            },
            content: params.content,
            attributes: params.attrs,
        };

        let _ = self.sender.send(Arc::new(ExecutionEvent::Log(entry)));

        Ok(())
    }
}

#[rquickjs::methods]
impl Logger {
    // maybe make this an actual tracer?
    // /// trace level log
    // fn trace(&self, _content: String, _metadata: rquickjs::Object<'_>, _ctx: Ctx<'_>) {
    //     self.log_impl(ctx, RunLogLevel::Trace, content, metadata)
    // }

    /// debug level log
    fn debug<'js>(&self, ctx: Ctx<'js>, params: LoggerParams) -> rquickjs::Result<()> {
        self.log_impl(ctx, EvalLogLevel::Debug, params)
    }

    /// info level log
    fn info<'js>(&self, ctx: Ctx<'js>, params: LoggerParams) -> rquickjs::Result<()> {
        self.log_impl(ctx, EvalLogLevel::Info, params)
    }

    /// warn level log
    fn warn<'js>(&self, ctx: Ctx<'js>, params: LoggerParams) -> rquickjs::Result<()> {
        self.log_impl(ctx, EvalLogLevel::Warning, params)
    }

    /// error level log
    fn error<'js>(&self, ctx: Ctx<'js>, params: LoggerParams) -> rquickjs::Result<()> {
        self.log_impl(ctx, EvalLogLevel::Error, params)
    }
}
