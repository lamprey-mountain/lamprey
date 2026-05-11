use common::v1::types::script::{
    Run, RunInput, RunLogEntry, RunLogLevel, RunLogSource, RunStatus, Script, ScriptInput,
    ScriptInputType, ScriptMetadata, ScriptVersion, ScriptVersionStatus,
};
use common::v1::types::util::Time;
use common::v1::types::{ChannelId, ConnectionId, MessageSync, RunId, ScriptId};
use dashmap::DashMap;
use rquickjs::async_with;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::error::Result;
use crate::{Error, ServerStateInner};

mod glue;
mod limits;
mod sync;
mod loader;

pub use sync::ScriptSyncer;

/// the service that manages all scripts
pub struct ServiceScripts {
    state: Arc<ServerStateInner>,

    /// per-channel runtimes
    runtimes: DashMap<ChannelId, Arc<ChannelRuntime>>,

    /// broadcast channels for script events per channel_id
    script_event_txs: DashMap<ChannelId, broadcast::Sender<MessageSync>>,
}

/// every script channel gets its own runtime
///
/// all scripts in any given script channel share the same runtime
struct ChannelRuntime {
    state: Arc<ServerStateInner>,
    channel_id: ChannelId,
    runtime: rquickjs::AsyncRuntime,

    // TODO: cache scripts
    scripts: HashMap<ScriptId, LoadedScript>,

    runs: Arc<DashMap<RunId, RunController>>,
    limits: limits::ChannelLimits,

    active_run: Arc<std::sync::Mutex<Option<RunId>>>,
    active_instruction_count: Arc<AtomicU64>,
}

/// a single script loaded in memory
struct LoadedScript {
    script_id: ScriptId,
    bytecode: Vec<u8>,
}

/// controls a script run
#[derive(Clone)]
pub struct RunController {
    context: rquickjs::AsyncContext,
    run: Arc<Run>,
    stop_signal: Arc<AtomicBool>,
}

impl ChannelRuntime {
    pub async fn new(
        state: Arc<ServerStateInner>,
        channel_id: ChannelId,
        limits: limits::ChannelLimits,
    ) -> Result<Self> {
        let rt = rquickjs::AsyncRuntime::new().unwrap();

        rt.set_memory_limit(limits.runtime.max_memory_bytes).await;
        rt.set_max_stack_size(limits.runtime.max_stack_size_bytes)
            .await;

        let active_instruction_count = Arc::new(AtomicU64::new(0));
        let count_clone = active_instruction_count.clone();
        let max_instructions = limits.run.max_instructions;

        let runs: Arc<DashMap<RunId, RunController>> = Arc::new(DashMap::new());
        let runs_clone = runs.clone();
        let active_run = Arc::new(std::sync::Mutex::new(None));
        let active_run_clone = active_run.clone();

        rt.set_interrupt_handler(Some(Box::new(move || {
            // check stop signal
            if let Ok(active) = active_run_clone.lock() {
                if let Some(run_id) = *active {
                    if let Some(ctl) = runs_clone.get(&run_id) {
                        if ctl.stop_signal.load(Ordering::Relaxed) {
                            return true;
                        }
                    } else {
                        // if the run was removed from the map, it should stop
                        return true;
                    }
                }
            }

            let count = count_clone.fetch_add(1, Ordering::Relaxed);
            count > max_instructions
        })))
        .await;

        Ok(Self {
            state,
            channel_id,
            runtime: rt,
            scripts: HashMap::new(),
            runs,
            limits,
            active_run,
            active_instruction_count,
        })
    }

    pub async fn load_script(
        &self,
        script_id: ScriptId,
        module_name: &str,
        module_source: &str,
    ) -> Result<LoadedScript> {
        let context = rquickjs::AsyncContext::full(&self.runtime).await.unwrap();

        let bytecode = async_with!(context => |ctx| {
            let module = rquickjs::Module::declare(ctx.clone(), module_name, module_source)?;
            let opts = rquickjs::WriteOptions::default();
            let bytes = module.write(opts).unwrap();

            rquickjs::Result::Ok(bytes)
        })
        .await
        .unwrap();

        self.runtime.idle().await;

        Ok(LoadedScript {
            script_id,
            bytecode,
        })
    }
}

#[derive(Debug, Default)]
pub struct ScriptExtracted {
    pub metadata: ScriptMetadata,
    pub inputs: Vec<ScriptInput>,
}

impl LoadedScript {
    /// extract the inputs/outputs this script supports
    pub async fn extract(&self, rt: &ChannelRuntime) -> Result<ScriptExtracted> {
        let context = rquickjs::AsyncContext::full(&rt.runtime).await.unwrap();

        let extracted = Arc::new(std::sync::Mutex::new(ScriptExtracted::default()));
        let extracted_for_ctx = extracted.clone();

        rt.active_instruction_count.store(0, Ordering::Relaxed);

        async_with!(context => |ctx| {
            // Set up the glue classes
            let _ = rquickjs::Class::<glue::register::ScriptRegister>::define(&ctx.globals());
            let _ = rquickjs::Class::<glue::register::InputBuilder>::define(&ctx.globals());

            // Set up __registry for callbacks and inputs (used by InputBuilder::run)
            let registry = rquickjs::Object::new(ctx.clone()).unwrap();
            let callbacks = rquickjs::Object::new(ctx.clone()).unwrap();
            let inputs = rquickjs::Array::new(ctx.clone()).unwrap();
            registry.set("callbacks", callbacks).unwrap();
            registry.set("inputs", inputs).unwrap();
            ctx.globals().set("__registry", registry).unwrap();

            let controller = rquickjs::Class::instance(ctx.clone(), glue::register::ScriptRegister {}).unwrap();

            // SAFETY: the bytecode was compiled ourselves in `load_script`
            let raw_module = unsafe { rquickjs::Module::load(ctx.clone(), &self.bytecode).unwrap() };
            let (module, _promise) = raw_module.eval().unwrap();

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

            if let Some(reg_val) = get_export("register") {
                if let Some(func) = reg_val.into_function() {
                    let _ = func.call::<_, ()>((controller, ));
                }
            }

            // extract registered inputs from __registry.inputs
            if let Ok(mut data) = extracted_for_ctx.lock() {
                if let Ok(registry) = ctx.globals().get::<_, rquickjs::Object>("__registry") {
                    if let Ok(inputs) = registry.get::<_, rquickjs::Array>("inputs") {
                        for i in 0..inputs.len() {
                            if let Ok(input_obj) = inputs.get::<rquickjs::Object>(i) {
                                let id: String = input_obj.get("id").unwrap_or_default();
                                let label: String = input_obj.get("label").unwrap_or_default();
                                data.inputs.push(ScriptInput {
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
                        data.metadata.name = name_str;
                    }
                } else {
                    data.metadata.name = "Untitled".to_string();
                }
            }

            rquickjs::Result::Ok(())
        })
        .await
        .unwrap();

        let extracted = Arc::into_inner(extracted)
            .expect("Arc should have no other owners")
            .into_inner()
            .expect("Mutex should not be poisoned");

        Ok(dbg!(extracted))
    }

    /// create a new run for this script
    pub async fn spawn(&self, rt: &ChannelRuntime, input: RunInput) -> Result<RunController> {
        let context = rquickjs::AsyncContext::full(&rt.runtime).await.unwrap();

        rt.active_instruction_count.store(0, Ordering::Relaxed);

        let run_id = RunId::new();
        let created_at = Time::now_utc();
        let state = rt.state.clone();
        let channel_id = rt.channel_id;
        let script_id = self.script_id;

        let run = Arc::new(Run {
            id: run_id,
            script_id,
            created_at,
            stopped_at: None,
            status: RunStatus::Creating,
            input: input.clone(),
        });
        state.data().script_run_create(&run).await?;
        let run_for_ctl = run.clone();

        let res = async_with!(context => |ctx| {
            let globals = ctx.globals();
            macro_rules! make_log_fn {
                ($level:expr, $state:expr) => {
                    rquickjs::Function::new(ctx.clone(), move |content_str: String, attrs_str: Option<String>| {
                        dbg!(&content_str);
                        dbg!(&attrs_str);
                        let content_json: serde_json::Value = serde_json::from_str(&content_str).unwrap_or(serde_json::Value::Null);

                        let content_str_final = if content_json.is_string() {
                            content_json.as_str().unwrap().to_string()
                        } else {
                            content_json.to_string()
                        };

                        let attrs = attrs_str.and_then(|json_str| {
                            let map: std::collections::HashMap<String, serde_json::Value> = serde_json::from_str(&json_str).ok()?;
                            let metadata = map.into_iter().map(|(k, v)| {
                                let v_str = if v.is_string() {
                                    v.as_str().unwrap().to_string()
                                } else {
                                    v.to_string()
                                };
                                (k, v_str)
                            }).collect::<std::collections::HashMap<String, String>>();
                            Some(common::v1::types::metadata::MessageMetadata(metadata))
                        }).unwrap_or_default();

                        let entry = RunLogEntry {
                            id: 0, // generated by DB
                            created_at: Time::now_utc(),
                            level: $level,
                            source: RunLogSource {
                                script_id,
                                run_id,
                                trace_id: None,
                                target: "script".to_string(),
                                span_start: 0,
                                span_end: 0,
                            },
                            content: content_str_final,
                            attributes: attrs,
                        };

                        let state_clone = $state.clone();
                        let entry_clone = entry.clone();
                        tokio::spawn(async move {
                            let mut data = state_clone.data();
                            let _ = data.script_log_insert(run_id, &entry_clone).await;
                            state_clone.services().scripts.broadcast(channel_id, MessageSync::ScriptLogCreate {
                                channel_id,
                                run_id,
                                entry: entry_clone,
                            }).await;
                        });
                    })
                }
            }

            let state_info = state.clone();
            globals.set("__log_info", make_log_fn!(RunLogLevel::Info, state_info)).unwrap();

            let state_warn = state.clone();
            globals.set("__log_warn", make_log_fn!(RunLogLevel::Warning, state_warn)).unwrap();

            let state_error = state.clone();
            globals.set("__log_error", make_log_fn!(RunLogLevel::Error, state_error)).unwrap();

            state.services().scripts.broadcast(channel_id, MessageSync::ScriptRunCreate {
                channel_id,
                run: (*run).clone(),
            }).await;

            // Set active run
            if let Ok(mut active) = rt.active_run.lock() {
                *active = Some(run_id);
            }

            ctx.eval::<(), _>(r#"
                globalThis.log = {
                    info: function(content, attrs) {
                        return __log_info(
                            content !== undefined ? JSON.stringify(content) : "null",
                            attrs !== undefined ? JSON.stringify(attrs) : null
                        );
                    },
                    warn: function(content, attrs) {
                        return __log_warn(
                            content !== undefined ? JSON.stringify(content) : "null",
                            attrs !== undefined ? JSON.stringify(attrs) : null
                        );
                    },
                    error: function(content, attrs) {
                        return __log_error(
                            content !== undefined ? JSON.stringify(content) : "null",
                            attrs !== undefined ? JSON.stringify(attrs) : null
                        );
                    }
                };
            "#)?;

            // update status to Active
            let mut data = state.data();
            let _ = data.script_run_update_status(run_id, RunStatus::Active).await;
            state.services().scripts.broadcast(channel_id, MessageSync::ScriptRunUpdate {
                channel_id,
                run: Run {
                    id: run_id,
                    script_id,
                    created_at,
                    stopped_at: None,
                    status: RunStatus::Active,
                    input: (*run).input.clone(),
                },
            }).await;

            // SAFETY: the bytecode was compiled ourselves in `load_script`
            let raw_module = unsafe { rquickjs::Module::load(ctx.clone(), &self.bytecode).unwrap() };
            let res = raw_module.eval();

            match res {
                Ok((module, _promise)) => {
                    // set up the registry for callbacks
                    let registry = rquickjs::Object::new(ctx.clone()).unwrap();
                    let callbacks = rquickjs::Object::new(ctx.clone()).unwrap();
                    let inputs = rquickjs::Array::new(ctx.clone()).unwrap();
                    registry.set("callbacks", callbacks).unwrap();
                    registry.set("inputs", inputs).unwrap();
                    ctx.globals().set("__registry", registry).unwrap();

                    // helper to get exports (named or from default object)
                    let get_export = |key: &str| -> Option<rquickjs::Value> {
                        if let Ok(val) = module.get::<_, rquickjs::Value>(key) {
                            if !val.is_undefined() && !val.is_null() {
                                return Some(val);
                            }
                        }
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

                    if let Some(reg_val) = get_export("register") {
                        if let Some(func) = reg_val.into_function() {
                            let _ = rquickjs::Class::<glue::register::ScriptRegister>::define(&ctx.globals());
                            let _ = rquickjs::Class::<glue::register::InputBuilder>::define(&ctx.globals());
                            let controller = rquickjs::Class::instance(ctx.clone(), glue::register::ScriptRegister {}).unwrap();
                            let _ = func.call::<_, ()>((controller, ));
                        }
                    }

                    // if we have a manual trigger, call the callback
                    let trigger_id = match &input {
                        RunInput::Trigger { id } => id.clone(),
                        RunInput::Extraction => "default".to_string(),
                    };
                    let registry = ctx.globals().get::<_, rquickjs::Object>("__registry").unwrap();
                    let callbacks = registry.get::<_, rquickjs::Object>("callbacks").unwrap();
                    if let Ok(func) = callbacks.get::<_, rquickjs::Function>(&trigger_id) {
                        let _ = func.call::<_, ()>(());
                    }

                    // Unset active run
                    if let Ok(mut active) = rt.active_run.lock() {
                        *active = None;
                    }

                    // update status to Exited
                    let mut data = state.data();
                    let _ = data.script_run_update_status(run_id, RunStatus::Exited).await;

                    rt.runs.remove(&run_id);

                    state.services().scripts.broadcast(channel_id, MessageSync::ScriptRunUpdate {
                        channel_id,
                        run: Run {
                            id: run_id,
                            script_id,
                            created_at,
                            stopped_at: Some(Time::now_utc()),
                            status: RunStatus::Exited,
                            input: (*run).input.clone(),
                        },
                    }).await;
                }
                Err(e) => {
                    // Unset active run
                    if let Ok(mut active) = rt.active_run.lock() {
                        *active = None;
                    }

                    // update status to Crashed
                    let mut data = state.data();
                    let _ = data.script_run_update_status(run_id, RunStatus::Crashed).await;

                    rt.runs.remove(&run_id);

                    state.services().scripts.broadcast(channel_id, MessageSync::ScriptRunUpdate {
                        channel_id,
                        run: Run {
                            id: run_id,
                            script_id,
                            created_at,
                            stopped_at: Some(Time::now_utc()),
                            status: RunStatus::Crashed,
                            input: (*run).input.clone(),
                        },
                    }).await;
                    return rquickjs::Result::Err(e);
                }
            }

            rquickjs::Result::Ok(())
        })
        .await;

        match res {
            Ok(_) => {
                // if it finished successfully (and wasn't async/didn't register hooks?)
                // actually, for many scripts they just register hooks and return.
                // so we probably shouldn't set it to Exited immediately if there are pending hooks.
                // but we don't have a way to know that yet.
            }
            Err(e) => {
                // already handled inside async_with for status update
                return Err(Error::BadRequest(format!("script evaluation failed: {e}")));
            }
        }

        Ok(RunController {
            context,
            run: run_for_ctl,
            stop_signal: Arc::new(AtomicBool::new(false)),
        })
    }
}

// TODO: automatically put runs to sleep to save memory
// TODO: automatically awaken runs when triggered
impl RunController {
    pub fn to_run(&self) -> Run {
        (*self.run).clone()
    }

    /// stop this run
    pub fn stop(&self) {
        self.stop_signal.store(true, Ordering::Relaxed);
    }

    // dubious apis, unsure about them
    // pub fn start(&self) {
    //     // status may be set to Borked
    //     // status may be set to Active
    //     // status may be set to Active
    //     todo!()
    // }

    // pub fn kill(&self) {
    //     // status may be set to Crashed(?)
    //     todo!()
    // }

    // /// serialize this run to disk
    // pub fn sleep(&self) {
    //     // status may be set to Sleeping
    //     todo!()
    // }

    // /// deserialize this run from disk
    // pub fn awaken() -> Self {
    //     // status may be set to Waking
    //     todo!()
    // }
}

impl ServiceScripts {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            runtimes: DashMap::new(),
            script_event_txs: DashMap::new(),
        }
    }

    /// get or create channel runtime, creating a broadcast channel if needed
    async fn init_rt(&self, channel_id: ChannelId) -> Result<Arc<ChannelRuntime>> {
        if let Some(rt) = self.runtimes.get(&channel_id) {
            return Ok(rt.clone());
        }

        let rt = Arc::new(
            ChannelRuntime::new(
                self.state.clone(),
                channel_id,
                self.state.config.scripts.limits.clone(),
            )
            .await?,
        );

        // create a broadcast channel for script events on this channel
        let (tx, _) = broadcast::channel(100);
        self.script_event_txs.insert(channel_id, tx);
        self.runtimes.insert(channel_id, rt.clone());
        Ok(rt)
    }

    /// clone a runtime reference for a given channel
    fn get_rt(&self, channel_id: &ChannelId) -> Option<Arc<ChannelRuntime>> {
        self.runtimes
            .get(channel_id)
            .map(|rt| Arc::clone(rt.value()))
    }

    /// broadcast a message sync event to all subscribers of a channel
    pub async fn broadcast(&self, channel_id: ChannelId, msg: MessageSync) {
        if let Some(entry) = self.script_event_txs.get(&channel_id) {
            let _ = entry.value().send(msg.clone());
        }

        // broadcast to the room as well
        let chan = self
            .state
            .services()
            .channels
            .get(channel_id, None)
            .await
            .ok();
        if let Some(room_id) = chan.and_then(|c| c.room_id) {
            let _ = self.state.broadcast_room2(room_id, msg).await;
        }
    }

    /// get or create a broadcast receiver for script events on a channel
    pub async fn subscribe_channel(
        &self,
        channel_id: ChannelId,
    ) -> Result<broadcast::Receiver<MessageSync>> {
        let rt = self.init_rt(channel_id).await?;
        drop(rt);
        if let Some(entry) = self.script_event_txs.get(&channel_id) {
            Ok(entry.value().subscribe())
        } else {
            let (tx, rx) = broadcast::channel(100);
            self.script_event_txs.insert(channel_id, tx);
            Ok(rx)
        }
    }

    /// create a script
    // TODO: process script (and script version) in background
    pub async fn create_script(&self, script: Script) -> Result<()> {
        let inputs = self.process(script.clone(), None).await?;
        let extracted_metadata = inputs.metadata;

        let mut data = self.state.data();

        // persist the script to the database
        data.script_create(&script).await?;

        // store the extracted inputs as cached_inputs on the version
        let inputs_json = serde_json::to_value(&inputs.inputs).ok();
        let version_id = data
            .script_version_create(
                script.id,
                script.channel_id,
                script.creator_id,
                script.latest_version.format.clone(),
                script.latest_version.location.clone(),
                extracted_metadata.clone(),
                inputs_json,
            )
            .await?;

        // update status to Valid
        data.script_version_update_status(script.id, version_id, ScriptVersionStatus::Valid)
            .await?;

        // update the script's latest_version metadata with extracted data
        let format = script.latest_version.format.clone();
        let location = script.latest_version.location.clone();
        data.script_update(script.id, format, location, extracted_metadata)
            .await?;

        // broadcast the newly created script
        if let Some(full_script) = data.script_get(script.id).await? {
            self.broadcast(
                script.channel_id,
                MessageSync::ScriptCreate {
                    script: full_script,
                },
            )
            .await;
        }

        Ok(())
    }

    /// create a script version
    pub async fn create_script_version(&self, script: Script, ver: ScriptVersion) -> Result<()> {
        let ver_format = ver.format.clone();
        let ver_location = ver.location.clone();
        let ver_metadata = ver.metadata.clone();
        let inputs = self.process(script.clone(), Some(ver)).await?;

        let mut data = self.state.data();

        // store the extracted inputs as cached_inputs on the new version
        let inputs_json = serde_json::to_value(&inputs.inputs).ok();
        let version_id = data
            .script_version_create(
                script.id,
                script.channel_id,
                script.creator_id,
                ver_format,
                ver_location,
                ver_metadata,
                inputs_json,
            )
            .await?;

        // update status to Valid
        data.script_version_update_status(script.id, version_id, ScriptVersionStatus::Valid)
            .await?;

        // broadcast the new version
        if let Some(full_ver) = data
            .script_version_get(script.id, script.channel_id, version_id)
            .await?
        {
            self.broadcast(
                script.channel_id,
                MessageSync::ScriptVersionCreate {
                    channel_id: script.channel_id,
                    script_id: script.id,
                    version: full_ver,
                },
            )
            .await;
        }

        Ok(())
    }

    /// process a script
    ///
    /// - does basic validation
    /// - extracts script inputs and metadata
    /// - optionally process a specific version of a script
    async fn process(&self, script: Script, ver: Option<ScriptVersion>) -> Result<ScriptExtracted> {
        let srv = self.state.services();

        let bytes = srv
            .media
            .download(
                ver.map_or_else(
                    || script.latest_version.location.media_id(),
                    |v| v.location.media_id(),
                )
                .unwrap(),
            )
            .await?;
        let source = str::from_utf8(&bytes)
            .map_err(|e| Error::BadRequest(format!("script is not valid utf-8: {e}")))?;

        let rt = self.init_rt(script.channel_id).await?;
        let loaded = rt.load_script(script.id, "strobbery", source).await?;
        let extracted = loaded.extract(&rt).await?;

        Ok(extracted)
    }

    /// create a new script syncer for a session
    pub fn create_syncer(&self, conn_id: ConnectionId) -> ScriptSyncer {
        ScriptSyncer::new(Arc::clone(&self.state), conn_id)
    }

    /// spawn a script
    pub async fn spawn(
        &self,
        channel_id: ChannelId,
        script_id: ScriptId,
        input: RunInput,
    ) -> Result<RunController> {
        let srv = self.state.services();
        let mut data = self.state.data();

        let script = data
            .script_get(script_id)
            .await?
            .ok_or(Error::BadStatic("script not found"))?;
        // TODO: verify the script status is Valid

        let bytes = srv
            .media
            .download(script.latest_version.location.media_id().unwrap())
            .await?;
        let source = str::from_utf8(&bytes).unwrap();

        let rt = self.init_rt(channel_id).await?;
        let script = rt.load_script(script_id, "strobbery", source).await?;
        let ctl = script.spawn(&rt, input).await?;

        // TODO: create a run record in the database

        rt.runs.insert(ctl.run.id, ctl.clone());

        Ok(ctl)
    }

    /// stop a script run
    pub async fn stop_run(
        &self,
        channel_id: ChannelId,
        _script_id: ScriptId,
        run_id: RunId,
    ) -> Result<()> {
        let rt = self.get_rt(&channel_id).ok_or(Error::NotFound)?;

        let ctl = rt.runs.remove(&run_id).ok_or(Error::NotFound)?;
        let (_, ctl) = ctl;
        ctl.stop();

        let mut data = self.state.data();
        let _ = data
            .script_run_update_status(run_id, RunStatus::Stopped)
            .await;

        self.broadcast(
            channel_id,
            MessageSync::ScriptRunUpdate {
                channel_id,
                run: Run {
                    id: run_id,
                    script_id: ctl.run.script_id,
                    created_at: ctl.run.created_at,
                    stopped_at: Some(Time::now_utc()),
                    status: RunStatus::Stopped,
                    input: ctl.run.input.clone(),
                },
            },
        )
        .await;

        Ok(())
    }
}
