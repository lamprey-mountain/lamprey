use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use async_trait::async_trait;
use common::v1::types::{
    script::{Run, RunInput, RunStatus, ScriptInput, ScriptInputType},
    util::Time,
    RunId, ScriptId,
};
use cpu_time::ProcessTime;
use dashmap::DashMap;
use rquickjs::{async_with, Ctx};
use tokio::sync::broadcast;
use tracing::error;

use crate::{
    engine::{ExecutionEvent, ScriptExtracted},
    limits::Limits,
    Error, ExecutionHandle, Executor, Result,
};

mod glue;
mod loader;

/// manager for all js executions
pub struct JsManager {
    /// limits per execution
    limits: Limits,

    // TODO: precompiled script cache
    scripts: DashMap<ScriptId, Arc<JsCompiledScript>>,
}

/// a single script loaded in memory
pub struct JsCompiledScript {
    script_id: ScriptId,
    bytecode: Vec<u8>,
}

/// state for javascript execution
pub struct JsExecutor {
    limits: Limits,
    script: Arc<JsCompiledScript>,
}

/// a handle to a live javascript execution
pub struct JsExecutionHandle {
    run: Arc<Run>,
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
        script_id: ScriptId,
        module_name: &str,
        module_source: &str,
    ) -> Result<JsExecutor> {
        // TODO: deduplicate runtime setup code
        let rt = rquickjs::AsyncRuntime::new()?;

        rt.set_memory_limit(self.limits.max_memory).await;
        rt.set_max_stack_size(512 * 1024).await;

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
            let module = rquickjs::Module::declare(ctx.clone(), module_name, module_source)?;
            let opts = rquickjs::WriteOptions::default();
            let bytes = module.write(opts)?;

            rquickjs::Result::Ok(bytes)
        })
        .await?;

        let script = Arc::new(JsCompiledScript {
            script_id,
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
    async fn spawn(&self, input: RunInput, run_id: RunId) -> Result<Box<dyn ExecutionHandle>> {
        // create new runtime + context for each run
        let rt = rquickjs::AsyncRuntime::new()?;
        rt.set_memory_limit(self.limits.max_memory).await;
        rt.set_max_stack_size(512 * 1024).await;

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

        let script_id = self.script.script_id;
        let created_at = Time::now_utc();
        let run = Arc::new(Run {
            id: run_id,
            script_id,
            created_at,
            stopped_at: None,
            status: RunStatus::Creating,
            input: input.clone(),
        });

        let script = Arc::clone(&self.script);

        tokio::spawn(async move {
            // keep runtime alive during execution
            let _rt_guard = rt;
            let events_sender_clone = events_sender.clone();

            let res = async_with!(context => |ctx| {
                match exec_inner(ctx.clone(), script_id, run_id, input, events_sender_clone, script, ext_send).await {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        if let Some(exception) = ctx.catch().into_object().and_then(rquickjs::Exception::from_object) {
                            error!(
                                %script_id,
                                %run_id,
                                message = %exception.message().unwrap_or_else(|| "Unknown JS error".to_string()),
                                stack = %exception.stack().unwrap_or_else(|| "No stack trace".to_string()),
                                "script javascript exception"
                            );
                        } else {
                            error!(%script_id, %run_id, "script runtime error: {:?}", err);
                        }
                        Err(err)
                    }
                }
            })
            .await;

            if let Err(err) = res {
                error!("script runtime error: {:?}", err);
                let _ = events_sender.send(Arc::new(ExecutionEvent::Status(RunStatus::Crashed)));
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
    script_id: ScriptId,
    run_id: RunId,
) -> Result<()> {
    let globals = ctx.globals();

    rquickjs::Class::<glue::register::ScriptRegister>::define(&globals)?;
    rquickjs::Class::<glue::register::InputBuilder>::define(&globals)?;

    globals.set("log", glue::log::Logger::new(sender, script_id, run_id))?;

    let registry = rquickjs::Object::new(ctx.clone())?;
    registry.set("callbacks", rquickjs::Object::new(ctx.clone())?)?;
    registry.set("inputs", rquickjs::Array::new(ctx.clone())?)?;
    globals.set("__registry", registry)?;

    Ok(())
}

async fn exec_inner<'js>(
    ctx: Ctx<'js>,
    script_id: ScriptId,
    run_id: RunId,
    input: RunInput,
    events_sender: broadcast::Sender<Arc<ExecutionEvent>>,
    script: Arc<JsCompiledScript>,
    ext_send: tokio::sync::watch::Sender<Option<ScriptExtracted>>,
) -> Result<()> {
    setup_environment(&ctx, events_sender.clone(), script_id, run_id)?;

    events_sender
        .send(Arc::new(ExecutionEvent::Status(RunStatus::Active)))
        .map_err(|e| Error::BroadcastSend(e.to_string()))?;

    // SAFETY: the bytecode was compiled ourselves in `load_script`
    let raw_module = unsafe { rquickjs::Module::load(ctx.clone(), &script.bytecode)? };
    let (module, promise) = raw_module.eval()?;
    promise.finish::<()>()?; // ensure top-level async code finishes

    if let Ok(reg_fn) = module.get::<_, rquickjs::Function>("register") {
        let controller = rquickjs::Class::instance(ctx.clone(), glue::register::ScriptRegister {})?;
        reg_fn.call::<_, ()>((controller,))?;
    }

    let mut extracted = ScriptExtracted::default();

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

    // extract registered inputs from __registry.inputs
    if let Ok(registry) = ctx.globals().get::<_, rquickjs::Object>("__registry") {
        if let Ok(inputs) = registry.get::<_, rquickjs::Array>("inputs") {
            for i in 0..inputs.len() {
                if let Ok(input_obj) = inputs.get::<rquickjs::Object>(i) {
                    let id: String = input_obj.get("id").unwrap_or_default();
                    let label: String = input_obj.get("label").unwrap_or_default();
                    extracted.inputs.push(ScriptInput {
                        id,
                        label,
                        ty: ScriptInputType::Manual,
                        effects: vec![],
                    });
                }
            }
        }
    }

    // extract some metadata
    if let Some(name_val) = get_export("name") {
        if let Ok(name_str) = name_val.get::<String>() {
            extracted.metadata.name = name_str;
        }
    } else {
        extracted.metadata.name = "Untitled".to_string();
    }

    if let Some(reg_val) = get_export("register") {
        if let Some(func) = reg_val.into_function() {
            let _ = rquickjs::Class::<glue::register::ScriptRegister>::define(&ctx.globals());
            let _ = rquickjs::Class::<glue::register::InputBuilder>::define(&ctx.globals());
            let controller =
                rquickjs::Class::instance(ctx.clone(), glue::register::ScriptRegister {})?;
            let _ = func.call::<_, ()>((controller,));
        }
    }

    match input {
        RunInput::Extraction => {
            // don't do anything just extract
        }
        RunInput::Trigger { id } => {
            let registry = ctx.globals().get::<_, rquickjs::Object>("__registry")?;
            let callbacks = registry.get::<_, rquickjs::Object>("callbacks")?;
            if let Ok(func) = callbacks.get::<_, rquickjs::Function>(&id) {
                let _ = func.call::<_, ()>(());
            }
        }
    }

    events_sender
        .send(Arc::new(ExecutionEvent::Status(RunStatus::Exited)))
        .map_err(|e| Error::BroadcastSend(e.to_string()))?;

    let _ = ext_send.send(Some(extracted));

    Ok(())
}

#[async_trait]
impl ExecutionHandle for JsExecutionHandle {
    fn run(&self) -> &Run {
        &*self.run
    }

    fn run_id(&self) -> RunId {
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

        if let Some(e) = &*self.ext_recv.borrow() {
            Ok(e.clone())
        } else {
            Err(Error::ExtractionDataMissing)
        }
    }

    fn clone_box(&self) -> Box<dyn ExecutionHandle> {
        Box::new(self.clone())
    }
}
