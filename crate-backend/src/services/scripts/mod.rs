use common::v1::types::script::{
    Run, RunLogEntry, RunLogLevel, RunLogSource, RunStatus, Script, ScriptInput, ScriptInputType,
    ScriptMetadata, ScriptVersion,
};
use common::v1::types::util::Time;
use common::v1::types::{ChannelId, ConnectionId, MessageSync, RunId, ScriptId, UserId};
use rquickjs::async_with;
use std::collections::{HashMap, VecDeque};
// use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::broadcast;

use crate::error::Result;
use crate::{Error, ServerStateInner};

mod glue;
mod limits;

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

    runs: HashMap<RunId, RunController>,
    limits: limits::ChannelLimits,

    active_instruction_count: Arc<std::sync::atomic::AtomicU64>,
}

/// a single script loaded in memory
struct LoadedScript {
    script_id: ScriptId,
    bytecode: Vec<u8>,
}

/// controls a script run
// what does this even do?
pub struct RunController {
    pub context: rquickjs::AsyncContext,
    pub run: Run,
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

        let active_instruction_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let count_clone = active_instruction_count.clone();
        let max_instructions = limits.run.max_instructions;

        rt.set_interrupt_handler(Some(Box::new(move || {
            let count = count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            count > max_instructions
        })))
        .await;

        // rt.memory_usage().await;
        // rt.set_host_promise_rejection_tracker(tracker);

        // rt.idle().await;

        Ok(Self {
            state,
            channel_id,
            runtime: rt,
            scripts: HashMap::new(),
            runs: HashMap::new(),
            limits,
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

pub enum ScriptInputData {
    Manual { id: String },
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

        rt.active_instruction_count
            .store(0, std::sync::atomic::Ordering::Relaxed);

        async_with!(context => |ctx| {
            // create the controller js object
            let controller = rquickjs::Object::new(ctx.clone()).unwrap();
            let ext = extracted_for_ctx.clone();
            controller.set("button", rquickjs::Function::new(ctx.clone(), move |id: String, label: Option<String>| {
                if let Ok(mut data) = ext.lock() {
                    data.inputs.push(ScriptInput {
                        id: id.clone(),
                        label: label.unwrap_or(id),
                        ty: ScriptInputType::Manual,
                        effects: vec![], // TODO: effect extraction
                    });
                }
            })).unwrap();

            // call register with the controller
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

            // extract some metadata
            if let Ok(mut data) = extracted_for_ctx.lock() {
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
    pub async fn spawn(
        &self,
        rt: &ChannelRuntime,
        _input: ScriptInputData,
    ) -> Result<RunController> {
        let context = rquickjs::AsyncContext::full(&rt.runtime).await.unwrap();

        rt.active_instruction_count
            .store(0, std::sync::atomic::Ordering::Relaxed);

        let run_id = RunId::new();
        let created_at = Time::now_utc();
        let state = rt.state.clone();
        let channel_id = rt.channel_id;
        let script_id = self.script_id;

        let run = Run {
            id: run_id,
            script_id,
            created_at,
            stopped_at: None,
            status: RunStatus::Creating,
        };
        state.data().script_run_create(&run).await?;

        async_with!(context => |ctx| {
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

                            let chan = state_clone.services().channels.get(channel_id, None).await.unwrap();
                            if let Some(room_id) = chan.room_id {
                                state_clone.broadcast_room2(
                                    room_id,
                                    MessageSync::ScriptLogCreate {
                                        channel_id,
                                        run_id,
                                        entry: entry_clone,
                                    },
                                ).await.unwrap();
                            }
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
            "#).unwrap();

            // SAFETY: the bytecode was compiled ourselves in `load_script`
            let raw_module = unsafe { rquickjs::Module::load(ctx.clone(), &self.bytecode).unwrap() };
            let (_module, _promise) = raw_module.eval().unwrap();

            rquickjs::Result::Ok(())
        })
        .await
        .unwrap();

        Ok(RunController {
            run: Run {
                id: run_id,
                script_id: self.script_id,
                created_at,
                stopped_at: None,
                status: RunStatus::Creating,
            },
            context,
        })
    }
}

// TODO: automatically put runs to sleep to save memory
// TODO: automatically awaken runs when triggered
impl RunController {
    pub fn to_run(&self) -> Run {
        self.run.clone()
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
            let _ = entry.value().send(msg);
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
        let _ = data
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

        // update the script's latest_version metadata with extracted data
        let format = script.latest_version.format.clone();
        let location = script.latest_version.location.clone();
        data.script_update(script.id, format, location, extracted_metadata)
            .await?;

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
        let _ = data
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
        let (query_tx, query_rx) = tokio::sync::watch::channel(None);
        ScriptSyncer {
            s: self.state.clone(),
            query_tx,
            query_rx,
            current_rx: None,
            conn_id,
            pending_sync: VecDeque::new(),
            user_id: None,
        }
    }

    /// spawn a script
    pub async fn spawn(
        &self,
        channel_id: ChannelId,
        script_id: ScriptId,
        input: ScriptInputData,
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

        Ok(ctl)
    }
}

// copied from document syncer, probably need to review this code
/// Handles script synchronization for a single client connection.
///
/// This struct manages the lifecycle of script subscriptions for a connection,
/// including subscribing/unsubscribing to script channels, broadcasting log
/// lines and metrics, and tracking run events.
pub struct ScriptSyncer {
    /// Reference to the server state for accessing services
    s: Arc<ServerStateInner>,

    /// Sends subscription requests to switch to a different script.
    /// When a client subscribes to a new script, the (channel_id, script_id) tuple is
    /// sent through this channel.
    query_tx: tokio::sync::watch::Sender<Option<(ChannelId, ScriptId)>>,

    /// Receives subscription requests from `query_tx`. The poll() loop monitors
    /// this receiver for changes. When a new query arrives, it sets up a
    /// subscription to the requested script and moves the subscription to `current_rx`.
    query_rx: tokio::sync::watch::Receiver<Option<(ChannelId, ScriptId)>>,

    /// The active script subscription. Contains the current (channel_id, script_id) tuple
    /// and a broadcast receiver for receiving script events (logs, metrics, runs).
    current_rx: Option<((ChannelId, ScriptId), broadcast::Receiver<MessageSync>)>,

    /// The connection ID associated with this syncer, used to filter out
    /// self-originated events.
    conn_id: ConnectionId,

    /// Queue of pending sync messages to be sent to the client.
    pending_sync: VecDeque<MessageSync>,

    /// The user ID of the authenticated user.
    user_id: Option<UserId>,
}

impl ScriptSyncer {
    pub async fn set_user_id(&mut self, user_id: Option<UserId>) {
        self.user_id = user_id;
    }

    /// Set the script to subscribe to.
    pub async fn set_context_id(&self, channel_id: ChannelId, script_id: ScriptId) -> Result<()> {
        self.query_tx
            .send(Some((channel_id, script_id)))
            .map_err(|_| Error::Internal("query channel closed".to_string()))?;
        Ok(())
    }

    /// Check if client is actively subscribed to a script.
    pub fn is_subscribed(&self, channel_id: &ChannelId, script_id: &ScriptId) -> bool {
        self.current_rx
            .as_ref()
            .map(|((current_channel, current_script), _)| {
                current_channel == channel_id && current_script == script_id
            })
            .unwrap_or(false)
    }

    pub async fn poll(&mut self) -> Result<MessageSync> {
        loop {
            if let Some(msg) = self.pending_sync.pop_front() {
                return Ok(msg);
            }

            if self.query_rx.has_changed().unwrap_or(false) {
                let _ = self.query_rx.borrow_and_update();
                let query = self.query_rx.borrow().clone();

                match query {
                    Some((channel_id, script_id)) => {
                        let rx = self
                            .s
                            .services()
                            .scripts
                            .subscribe_channel(channel_id)
                            .await?;
                        self.current_rx = Some(((channel_id, script_id), rx));

                        return Ok(MessageSync::ScriptSubscribed {
                            channel_id,
                            script_id,
                            connection_id: self.conn_id,
                        });
                    }
                    None => {
                        self.current_rx = None;
                        continue;
                    }
                }
            }

            if let Some(((_channel_id, _script_id), rx)) = &mut self.current_rx {
                tokio::select! {
                    res = rx.recv() => {
                        match res {
                            Ok(msg) => {
                                return Ok(msg);
                            }
                            Err(_) => continue,
                        }
                    }
                    _ = self.query_rx.changed() => continue,
                }
            } else {
                self.query_rx
                    .changed()
                    .await
                    .map_err(|_| Error::Internal("query channel closed".to_string()))?;
                continue;
            }
        }
    }
}
