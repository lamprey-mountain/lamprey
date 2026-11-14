use std::result::Result as StdResult;
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
        Eval, EvalInput, EvalLogEntry, EvalLogLevel, EvalLogSource, EvalStatus, RedexHandlerType,
        metadata::{License, Semver},
    },
    util::Time,
};
use cpu_time::ProcessTime;
use dashmap::DashMap;
use futures::{FutureExt, future::Shared};
use rquickjs::promise::MaybePromise;
use rquickjs::{Ctx, Exception, FromJs, Promise};
use tokio::sync::{broadcast, oneshot};
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
    ext_recv: Shared<oneshot::Receiver<StdResult<ScriptExtracted, Arc<Error>>>>,
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
        let bytecode = context
            .with(|ctx: Ctx<'_>| {
                let module = rquickjs::Module::declare(ctx.clone(), module_name, module_source)?;
                let opts = rquickjs::WriteOptions::default();
                let bytes = module.write(opts)?;
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
        let (ext_send, ext_recv) = oneshot::channel();

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
            // rt.set_host_promise_rejection_tracker(tracker);
            // rt.set_promise_hook(Some(Box::new(|_ctx, _hook_type, _promise, _parent| {
            //     // TODO
            // })));

            // keep runtime alive during execution
            let _rt_guard = rt.clone();
            let events_sender_clone = events_sender.clone();

            let extracted = exec_inner(
                &rt,
                &context,
                redex_id,
                input,
                events_sender_clone,
                script,
                start_time_wall + max_cpu_wall,
            )
            .await;

            if let Err(err) = &extracted {
                error!("eval runtime error: {:?}", err);

                let log = EvalLogEntry {
                    id: 0,
                    created_at,
                    level: EvalLogLevel::Error,
                    source: EvalLogSource::Runtime,
                    content: err.to_string(),
                    attributes: Default::default(),
                };
                let _ = events_sender.send(Arc::new(ExecutionEvent::Log(log)));

                let _ = events_sender.send(Arc::new(ExecutionEvent::Status(EvalStatus::Crashed)));
            }

            // TODO: error handling
            let _ = ext_send.send(extracted.map_err(Arc::new));
        });

        let handle = JsExecutionHandle {
            run,
            stop_signal,
            events: events_receiver,
            ext_recv: ext_recv.shared(),
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
    rquickjs::Class::<glue::url::JsUrl>::define(&globals)?;
    rquickjs::Class::<glue::blob::Blob>::define(&globals)?;
    rquickjs::Class::<glue::blob::File>::define(&globals)?;

    // File extends Blob in the web spec
    if let (Some(blob), Some(file)) = (
        rquickjs::Class::<glue::blob::Blob>::prototype(ctx)?,
        rquickjs::Class::<glue::blob::File>::prototype(ctx)?,
    ) {
        file.set_prototype(Some(&blob))?;
    }

    #[cfg(feature = "crypto")]
    glue::crypto::setup(ctx)?;
    glue::abort::setup(ctx)?;
    glue::streams::setup(ctx)?;
    glue::encoding::setup(ctx)?;

    globals.set("log", glue::log::Logger::new(sender, script_id))?;

    Ok(())
}

/// run a single eval of a javascript redex
///
/// the run is executed as a task owned by the runtime, so it can suspend on host futures
/// (ie. network requests) while this function keeps driving the runtime
async fn exec_inner(
    rt: &rquickjs::AsyncRuntime,
    context: &rquickjs::AsyncContext,
    script_id: RedexId,
    input: EvalInput,
    events_sender: broadcast::Sender<Arc<ExecutionEvent>>,
    script: Arc<JsCompiledScript>,
    deadline: Instant,
) -> Result<ScriptExtracted> {
    let (result_tx, mut result_rx) = oneshot::channel::<Result<ScriptExtracted>>();

    context
        .with(|ctx| -> Result<()> {
            setup_environment(&ctx, events_sender.clone(), script_id)?;

            events_sender
                .send(Arc::new(ExecutionEvent::Status(EvalStatus::Active)))
                .map_err(|e| Error::BroadcastSend(e.to_string()))?;

            // SAFETY: the bytecode was compiled ourselves in `load`
            let raw_module = unsafe { rquickjs::Module::load(ctx.clone(), &script.bytecode)? };
            let (module, eval_promise) = raw_module.eval()?;

            // the run is resumed once this closure releases the runtime lock,
            // which is what lets handlers await promises resolved by host futures
            let run_ctx = ctx.clone();
            let run_events_sender = events_sender.clone();
            ctx.spawn(async move {
                let result =
                    run_handlers(run_ctx, module, eval_promise, input, &run_events_sender).await;
                let _ = result_tx.send(result);
            });

            Ok(())
        })
        .await?;

    // drive pending jobs and host futures until the run settles
    let drive = async {
        tokio::select! {
            result = &mut result_rx => Some(result),
            _ = rt.idle() => None,
        }
    };

    match tokio::time::timeout_at(tokio::time::Instant::from(deadline), drive).await {
        Ok(Some(Ok(result))) => result,
        Ok(Some(Err(_))) => Err(Error::OneshotRecv("run task was dropped".to_string())),
        // the runtime is idle but the run never produced a result
        Ok(None) => match result_rx.try_recv() {
            Ok(result) => result,
            Err(_) => Err(Error::RuntimeError {
                message: "run ended without a result".to_string(),
                stack: String::new(),
            }),
        },
        Err(_) => Err(Error::RuntimeError {
            message: "run timed out".to_string(),
            stack: String::new(),
        }),
    }
}

/// wait for a value to settle if it is a promise
async fn settle<'js>(ctx: &Ctx<'js>, value: rquickjs::Value<'js>) -> Result<rquickjs::Value<'js>> {
    MaybePromise::from_value(value)
        .into_future::<rquickjs::Value<'js>>()
        .await
        .map_err(|err| js_error(ctx, err))
}

/// wait for a module's top level await to finish
async fn settle_module<'js>(ctx: &Ctx<'js>, eval: Promise<'js>) -> Result<()> {
    eval.into_future::<()>()
        .await
        .map_err(|err| js_error(ctx, err))
}

/// turn a pending javascript exception into an error
fn js_error(ctx: &Ctx<'_>, err: rquickjs::Error) -> Error {
    if matches!(err, rquickjs::Error::Exception) {
        if let Some(exception) = ctx.catch().into_object().and_then(Exception::from_object) {
            return Error::from_exception(exception);
        }
    }

    Error::Rquickjs(err)
}

async fn run_handlers<'js>(
    ctx: Ctx<'js>,
    module: rquickjs::Module<'js, rquickjs::module::Evaluated>,
    eval_promise: Promise<'js>,
    input: EvalInput,
    events_sender: &broadcast::Sender<Arc<ExecutionEvent>>,
) -> Result<ScriptExtracted> {
    settle_module(&ctx, eval_promise).await?;

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
            let result = reg_fn.call::<_, rquickjs::Value>((controller,))?;
            settle(&ctx, result).await?;
        } else {
            // TODO: send warn or error to user
        }
    }

    {
        let registry = registry.lock().unwrap();
        for input in &registry.inputs {
            extracted.inputs.push(input.definition.clone());
        }
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

    match input {
        EvalInput::Extraction => {
            // don't do anything just extract
        }
        EvalInput::Manual { id, .. } => {
            let callback = registry
                .lock()
                .unwrap()
                .find(&id)
                .map(|input| input.callback.clone());

            if let Some(callback) = callback {
                let handler = callback.restore(&ctx)?;
                let result = handler.call::<_, rquickjs::Value>(())?;
                let _ = settle(&ctx, result).await;
            }
        }
        EvalInput::Http { request } => {
            let callback = registry
                .lock()
                .unwrap()
                .inputs
                .iter()
                .find(|input| matches!(input.definition.ty, RedexHandlerType::Http))
                .map(|input| input.callback.clone());

            if let Some(callback) = callback {
                let handler = callback.restore(&ctx)?;

                let response = handler.call::<_, rquickjs::Value>((glue::http::Request {
                    method: request.method().to_string(),
                    url: request.uri().to_string(),
                    headers: request.headers().to_owned(),
                    body: request.into_body(),
                },))?;

                let response = settle(&ctx, response).await?;
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
            let callbacks: Vec<_> = registry
                .lock()
                .unwrap()
                .inputs
                .iter()
                .filter(|input| matches!(input.definition.ty, RedexHandlerType::Event))
                .map(|input| input.callback.clone())
                .collect();

            for callback in callbacks {
                let handler = callback.restore(&ctx)?;

                let js_event = rquickjs_serde::to_value(ctx.clone(), &*event).map_err(|e| {
                    rquickjs::Error::new_from_js_message("object", "MessageSync", e.to_string())
                })?;

                // TODO: error handling
                let result = handler.call::<_, rquickjs::Value>((js_event,))?;
                let _ = settle(&ctx, result).await;
            }
        }
    }

    events_sender
        .send(Arc::new(ExecutionEvent::Status(EvalStatus::Exited)))
        .map_err(|e| Error::BroadcastSend(e.to_string()))?;

    Ok(extracted)
}

#[async_trait]
impl ExecutionHandle for JsExecutionHandle {
    fn eval(&self) -> &Eval {
        &*self.run
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

    async fn done(&self) -> Result<ScriptExtracted> {
        match self.ext_recv.clone().await {
            Ok(Ok(extracted)) => Ok(extracted),
            Ok(Err(err)) => match &*err {
                Error::RuntimeError { message, stack } => Err(Error::RuntimeError {
                    message: message.to_string(),
                    stack: stack.to_string(),
                }),
                _ => Err(Error::Other(err.to_string())),
            },
            Err(e) => Err(Error::OneshotRecv(e.to_string())),
        }
    }

    fn clone_box(&self) -> Box<dyn ExecutionHandle> {
        Box::new(self.clone())
    }
}
