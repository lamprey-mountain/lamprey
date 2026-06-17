use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use async_trait::async_trait;
use common::v1::types::{
    EvalId, RedexId, RedexVerId,
    redex::{
        Eval, EvalInput, EvalStatus, RedexHandlerType,
        metadata::{License, Semver},
    },
    util::Time,
};
use cpu_time::ProcessTime;
use dashmap::DashMap;
use rquickjs::{Ctx, FromJs, async_with};
use tokio::sync::broadcast;
use tracing::error;

use crate::{
    Error, ExecutionHandle, Executor, Result,
    engine::{ExecutionEvent, ScriptExtracted},
    javascript::{
        glue::register::ScriptRegistry,
        loader::{ModuleLoader, ModuleResolver},
    },
    limits::Limits,
};

mod glue;
mod loader;
// mod record;

/// manager for all js executions
pub struct JsManager {
    /// limits per execution
    limits: Limits,

    // TODO: precompiled script cache
    scripts: DashMap<RedexId, Arc<JsCompiledScript>>,
}

/// a single script loaded in memory
pub struct JsCompiledScript {
    redex_id: RedexId,
    redex_version_id: RedexVerId,
    bytecode: Vec<u8>,
}

/// state for javascript execution
pub struct JsExecutor {
    limits: Limits,
    script: Arc<JsCompiledScript>,
    // replay: Replay,
}

/// a handle to a live javascript execution
pub struct JsExecutionHandle {
    run: Arc<Eval>,
    stop_signal: Arc<AtomicBool>,
    events: broadcast::Receiver<Arc<ExecutionEvent>>,
    ext_recv: tokio::sync::watch::Receiver<Option<ScriptExtracted>>,
}

impl JsExecutionHandle {
    fn clone(&self) -> Self {
        Self {
            run: self.run.clone(),
            stop_signal: self.stop_signal.clone(),
            events: self.events.resubscribe(),
            ext_recv: self.ext_recv.clone(),
        }
    }
}

impl JsManager {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            scripts: DashMap::new(),
        }
    }

    /// load a js script
    pub async fn load(
        &self,
        script_id: RedexId,
        script_version_id: RedexVerId,
        module_name: &str,
        module_source: &str,
    ) -> Result<JsExecutor> {
        // TODO: deduplicate runtime setup code
        let rt = rquickjs::AsyncRuntime::new()?;

        rt.set_memory_limit(self.limits.max_memory).await;
        rt.set_max_stack_size(512 * 1024).await;
        rt.set_loader(ModuleResolver::new(), ModuleLoader::new())
            .await;

        let start_time_wall = Instant::now();
        let start_time_process = ProcessTime::now();
        let max_cpu_wall = self.limits.max_cpu_wall;
        let max_cpu_process = self.limits.max_cpu_process;

        rt.set_interrupt_handler(Some(Box::new(move || {
            if start_time_wall.elapsed() > max_cpu_wall {
                return true;
            }

            if start_time_process.elapsed() > max_cpu_process {
                return true;
            }

            false
        })))
        .await;

        // TODO: try to reuse cache
        let context = rquickjs::AsyncContext::full(&rt).await?;
        let bytecode = async_with!(context => |ctx| {
            let module = dbg!(rquickjs::Module::declare(ctx.clone(), module_name, module_source))?;
            let opts = rquickjs::WriteOptions::default();
            let bytes = dbg!(module.write(opts))?;

            rquickjs::Result::Ok(bytes)
        })
        .await?;

        let script = Arc::new(JsCompiledScript {
            redex_id: script_id,
            redex_version_id: script_version_id,
            bytecode,
        });
        self.scripts.insert(script_id, Arc::clone(&script));
        // TODO: cleanup cache

        Ok(JsExecutor {
            limits: self.limits.clone(),
            script,
        })
    }
}

#[async_trait]
impl Executor for JsExecutor {
    async fn spawn(&self, input: EvalInput, eval_id: EvalId) -> Result<Box<dyn ExecutionHandle>> {
        // create new runtime + context for each run
        let rt = rquickjs::AsyncRuntime::new()?;
        rt.set_memory_limit(self.limits.max_memory).await;
        rt.set_max_stack_size(512 * 1024).await;
        rt.set_loader(ModuleResolver::new(), ModuleLoader::new())
            .await;

        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();

        let start_time_wall = Instant::now();
        let start_time_process = ProcessTime::now();
        let max_cpu_wall = self.limits.max_cpu_wall;
        let max_cpu_process = self.limits.max_cpu_process;

        rt.set_interrupt_handler(Some(Box::new(move || {
            if start_time_wall.elapsed() > max_cpu_wall {
                return true;
            }

            if start_time_process.elapsed() > max_cpu_process {
                return true;
            }

            if stop_signal_clone.load(Ordering::Relaxed) {
                return true;
            }

            false
        })))
        .await;

        let context = rquickjs::AsyncContext::full(&rt).await?;

        // new events channel per run too
        let (events_sender, events_receiver) = broadcast::channel::<Arc<ExecutionEvent>>(100);
        let (ext_send, ext_recv) = tokio::sync::watch::channel(None);

        let redex_id = self.script.redex_id;
        let redex_version_id = self.script.redex_version_id;
        let created_at = Time::now_utc();
        let run = Arc::new(Eval {
            id: eval_id,
            redex_id,
            redex_version_id,
            created_at,
            stopped_at: None,
            status: EvalStatus::Creating,
            input: input.clone().into(),
        });

        let script = Arc::clone(&self.script);

        tokio::spawn(async move {
            // keep runtime alive during execution
            let _rt_guard = rt;
            let events_sender_clone = events_sender.clone();

            let res = async_with!(context => |ctx| {
                match exec_inner(ctx.clone(), redex_id, input, events_sender_clone, script, ext_send).await {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        if let Some(exception) = ctx.catch().into_object().and_then(rquickjs::Exception::from_object) {
                            error!(
                                %redex_id,
                                %eval_id,
                                message = %exception.message().unwrap_or_else(|| "Unknown JS error".to_string()),
                                stack = %exception.stack().unwrap_or_else(|| "No stack trace".to_string()),
                                "eval javascript exception"
                            );
                            Err(Error::from_exception(exception))
                        } else {
                            error!(%redex_id, %eval_id, "eval error: {:?}", err);
                            Err(err)
                        }
                    }
                }
            })
            .await;

            if let Err(err) = res {
                error!("eval runtime error: {:?}", err);
                let _ = events_sender.send(Arc::new(ExecutionEvent::Status(EvalStatus::Crashed)));
            }
        });

        let handle = JsExecutionHandle {
            run,
            stop_signal,
            events: events_receiver,
            ext_recv,
        };

        Ok(Box::new(handle))
    }
}

fn setup_environment(
    ctx: &Ctx<'_>,
    sender: broadcast::Sender<Arc<ExecutionEvent>>,
    script_id: RedexId,
) -> Result<()> {
    let globals = ctx.globals();

    rquickjs::Class::<glue::register::ScriptRegister>::define(&globals)?;
    rquickjs::Class::<glue::register::InputBuilder>::define(&globals)?;

    globals.set("log", glue::log::Logger::new(sender, script_id))?;

    Ok(())
}

async fn exec_inner<'js>(
    ctx: Ctx<'js>,
    script_id: RedexId,
    input: EvalInput,
    events_sender: broadcast::Sender<Arc<ExecutionEvent>>,
    script: Arc<JsCompiledScript>,
    ext_send: tokio::sync::watch::Sender<Option<ScriptExtracted>>,
) -> Result<()> {
    setup_environment(&ctx, events_sender.clone(), script_id)?;

    events_sender
        .send(Arc::new(ExecutionEvent::Status(EvalStatus::Active)))
        .map_err(|e| Error::BroadcastSend(e.to_string()))?;

    // SAFETY: the bytecode was compiled ourselves in `load_script`
    let raw_module = unsafe { rquickjs::Module::load(ctx.clone(), &script.bytecode)? };
    let (module, promise) = raw_module.eval()?;
    promise.finish::<()>()?; // ensure top-level async code finishes

    let registry = Arc::new(Mutex::new(ScriptRegistry::new()));
    let mut extracted = ScriptExtracted::new("unnamed".to_owned());

    // helper to get exports (named or from default object)
    let get_export = |key: &str| -> Option<rquickjs::Value> {
        // check for named export (ie. export const foo = ...)
        if let Ok(val) = module.get::<_, rquickjs::Value>(key) {
            if !val.is_undefined() && !val.is_null() {
                return Some(val);
            }
        }

        // check inside default export (ie. export default { foo: ... })
        if let Ok(default_val) = module.get::<_, rquickjs::Value>("default") {
            if let Some(obj) = default_val.as_object() {
                if let Ok(val) = obj.get::<_, rquickjs::Value>(key) {
                    if !val.is_undefined() && !val.is_null() {
                        return Some(val);
                    }
                }
            }
        }

        None
    };

    if let Some(reg_fn) = get_export("register") {
        if let Some(reg_fn) = reg_fn.into_function() {
            let controller = rquickjs::Class::instance(
                ctx.clone(),
                glue::register::ScriptRegister {
                    registry: Arc::clone(&registry),
                },
            )?;
            reg_fn.call::<_, ()>((controller,))?;
        } else {
            // TODO: send warn or error to user
        }
    }

    let r = registry.lock().unwrap();
    for input in dbg!(&r.inputs) {
        extracted.inputs.push(input.definition.clone());
    }

    // extract some metadata
    if let Some(name_val) = get_export("name") {
        if let Ok(name_str) = name_val.get::<String>() {
            extracted.metadata.name = name_str;
        } else {
            extracted.metadata.name = "Untitled".to_string();
        }
    } else {
        extracted.metadata.name = "Untitled".to_string();
    }

    if let Some(desc_val) = get_export("description") {
        if let Ok(desc_str) = desc_val.get::<String>() {
            extracted.metadata.description = Some(desc_str);
        }
    }

    if let Some(version_val) = get_export("version") {
        if let Ok(version_str) = version_val.get::<String>() {
            extracted.metadata.version = Some(Semver(version_str));
        }
    }

    if let Some(license_val) = get_export("license") {
        if let Ok(license_str) = license_val.get::<String>() {
            extracted.metadata.license = Some(License(license_str));
        }
    }

    match dbg!(input) {
        EvalInput::Extraction => {
            // don't do anything just extract
        }
        EvalInput::Manual { id, .. } => {
            if let Some(input) = r.find(&id) {
                let _ = input.callback.clone().restore(&ctx)?.call::<_, ()>(());
            }
        }
        EvalInput::Http { request } => {
            if let Some(input) = r
                .inputs
                .iter()
                .find(|i| i.definition.ty == RedexHandlerType::Http {})
            {
                let handler = input.callback.clone().restore(&ctx)?;

                let response: rquickjs::Value = handler.call((glue::http::Request {
                    method: request.method().to_string(),
                    url: request.uri().to_string(),
                    headers: request.headers().to_owned(),
                    body: request.into_body(),
                },))?;

                let response: rquickjs::Value = match response.try_into_promise() {
                    Ok(p) => p.finish()?,
                    Err(val) => val,
                };

                let response = glue::http::Response::from_js(&ctx, response)?;

                let mut builder = http::Response::builder().status(response.status);
                if let Some(h) = builder.headers_mut() {
                    *h = response.headers;
                }
                let response = builder.body(response.body).unwrap();

                events_sender
                    .send(Arc::new(ExecutionEvent::HttpResponse(response)))
                    .map_err(|e| Error::BroadcastSend(e.to_string()))?;
            }
        }
        EvalInput::Event { event } => {
            for input in r
                .inputs
                .iter()
                .filter(|i| i.definition.ty == RedexHandlerType::Event)
            {
                let handler = input.callback.clone().restore(&ctx)?;

                let js_event = rquickjs_serde::to_value(ctx.clone(), &*event).map_err(|e| {
                    rquickjs::Error::new_from_js_message("object", "MessageSync", e.to_string())
                })?;

                // TODO: error handling
                let _ = handler.call::<_, ()>((js_event,));
            }
        }
    }

    // TODO: error handling
    let _ = ext_send.send(Some(extracted));

    events_sender
        .send(Arc::new(ExecutionEvent::Status(EvalStatus::Exited)))
        .map_err(|e| Error::BroadcastSend(e.to_string()))?;

    Ok(())
}

#[async_trait]
impl ExecutionHandle for JsExecutionHandle {
    fn eval(&self) -> &Eval {
        &*self.run
    }

    fn eval_id(&self) -> EvalId {
        self.run.id
    }

    fn stop(&self) {
        self.stop_signal.store(true, Ordering::Relaxed);
    }

    async fn poll(&mut self) -> Result<Arc<ExecutionEvent>> {
        self.events
            .recv()
            .await
            .map_err(|e| Error::BroadcastRecv(e.to_string()))
    }

    async fn done(&mut self) -> Result<ScriptExtracted> {
        self.ext_recv
            .changed()
            .await
            .map_err(|e| Error::WatchChanged(e.to_string()))?;

        if let Some(e) = dbg!(&*self.ext_recv.borrow()) {
            Ok(e.clone())
        } else {
            Err(Error::ExtractionDataMissing)
        }
    }

    fn clone_box(&self) -> Box<dyn ExecutionHandle> {
        Box::new(self.clone())
    }
}
