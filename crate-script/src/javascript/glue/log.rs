use std::{collections::HashMap, sync::Arc};

use common::v1::types::{
    metadata::MessageMetadata,
    script::{RunLogEntry, RunLogLevel, RunLogSource},
    util::Time,
    RunId, ScriptId,
};
use rquickjs::{
    class::{Trace, Tracer},
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
    script_id: ScriptId,
    run_id: RunId,
}

// none of these fields need to be traced
impl<'js> Trace<'js> for Logger {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

impl Logger {
    pub(crate) fn new(
        sender: Sender<Arc<ExecutionEvent>>,
        script_id: ScriptId,
        run_id: RunId,
    ) -> Self {
        Self {
            sender,
            script_id,
            run_id,
        }
    }
}

#[rquickjs::methods]
impl Logger {
    // maybe make this an actual tracer?
    // /// trace level log
    // fn trace(&self, _content: String, _metadata: rquickjs::Object<'_>, _ctx: Ctx<'_>) {
    //     todo!()
    // }

    /// debug level log
    fn debug<'js>(
        &self,
        content: String,
        metadata: rquickjs::Value<'js>,
        ctx: Ctx<'js>,
    ) -> rquickjs::Result<()> {
        let attrs: HashMap<String, String> = HashMap::from_js(&ctx, metadata)?;
        let attrs = MessageMetadata(attrs);
        attrs.validate().map_err(|err| {
            rquickjs::Error::new_from_js_message("object", "MessageMetadata", err.to_string())
        })?;

        let entry = RunLogEntry {
            id: 0, // will be filled in by db
            created_at: Time::now_utc(),
            level: RunLogLevel::Debug,
            source: RunLogSource {
                script_id: self.script_id,
                run_id: self.run_id,
                trace_id: None,
                target: "script".to_string(),
                span_start: 0, // TODO
                span_end: 0,   // TODO
            },
            content,
            attributes: attrs,
        };

        let _ = self.sender.send(Arc::new(ExecutionEvent::Log(entry)));

        Ok(())
    }

    /// info level log
    fn info<'js>(
        &self,
        content: String,
        metadata: rquickjs::Value<'js>,
        ctx: Ctx<'js>,
    ) -> rquickjs::Result<()> {
        let attrs: HashMap<String, String> = HashMap::from_js(&ctx, metadata)?;
        let attrs = MessageMetadata(attrs);
        attrs.validate().map_err(|err| {
            rquickjs::Error::new_from_js_message("object", "MessageMetadata", err.to_string())
        })?;

        let entry = RunLogEntry {
            id: 0, // will be filled in by db
            created_at: Time::now_utc(),
            level: RunLogLevel::Info,
            source: RunLogSource {
                script_id: self.script_id,
                run_id: self.run_id,
                trace_id: None,
                target: "script".to_string(),
                span_start: 0, // TODO
                span_end: 0,   // TODO
            },
            content,
            attributes: attrs,
        };

        let _ = self.sender.send(Arc::new(ExecutionEvent::Log(entry)));

        Ok(())
    }

    /// warn level log
    fn warn<'js>(
        &self,
        content: String,
        metadata: rquickjs::Value<'js>,
        ctx: Ctx<'js>,
    ) -> rquickjs::Result<()> {
        let attrs: HashMap<String, String> = HashMap::from_js(&ctx, metadata)?;
        let attrs = MessageMetadata(attrs);
        attrs.validate().map_err(|err| {
            rquickjs::Error::new_from_js_message("object", "MessageMetadata", err.to_string())
        })?;

        let entry = RunLogEntry {
            id: 0, // will be filled in by db
            created_at: Time::now_utc(),
            level: RunLogLevel::Warning,
            source: RunLogSource {
                script_id: self.script_id,
                run_id: self.run_id,
                trace_id: None,
                target: "script".to_string(),
                span_start: 0, // TODO
                span_end: 0,   // TODO
            },
            content,
            attributes: attrs,
        };

        let _ = self.sender.send(Arc::new(ExecutionEvent::Log(entry)));

        Ok(())
    }

    /// error level log
    fn error<'js>(
        &self,
        content: String,
        metadata: rquickjs::Value<'js>,
        ctx: Ctx<'js>,
    ) -> rquickjs::Result<()> {
        let attrs: HashMap<String, String> = HashMap::from_js(&ctx, metadata)?;
        let attrs = MessageMetadata(attrs);
        attrs.validate().map_err(|err| {
            rquickjs::Error::new_from_js_message("object", "MessageMetadata", err.to_string())
        })?;

        let entry = RunLogEntry {
            id: 0, // will be filled in by db
            created_at: Time::now_utc(),
            level: RunLogLevel::Error,
            source: RunLogSource {
                script_id: self.script_id,
                run_id: self.run_id,
                trace_id: None,
                target: "script".to_string(),
                span_start: 0, // TODO
                span_end: 0,   // TODO
            },
            content,
            attributes: attrs,
        };

        let _ = self.sender.send(Arc::new(ExecutionEvent::Log(entry)));

        Ok(())
    }
}
