#![allow(dead_code)] // TEMP: while prototyping

use common::v1::types::script::{
    Run, RunStatus, Script, ScriptInput, ScriptInputType, ScriptMetadata, ScriptVersion,
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
    runtime: rquickjs::AsyncRuntime,

    // TODO: cache scripts
    scripts: HashMap<ScriptId, LoadedScript>,

    runs: HashMap<RunId, RunController>,
    // TODO: per-channel limits
    // limits: limits::Limits,
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
    pub async fn new() -> Result<Self> {
        let rt = rquickjs::AsyncRuntime::new().unwrap();

        // TODO: move these to ~~consts~~ limits
        rt.set_memory_limit(8 * 1024 * 1024).await;
        rt.set_max_stack_size(512 * 1024).await;

        // rt.memory_usage().await;
        // rt.set_host_promise_rejection_tracker(tracker);

        // let instruction_count = Arc::new(AtomicU64::new(0));
        // let count_clone = instruction_count.clone();

        // rt.set_interrupt_handler(Some(Box::new(move || {
        //     let count = count_clone.fetch_add(1, Ordering::Relaxed);

        //     // interrupt js at 1 million instructions
        //     // TODO: move to const
        //     // count > 1_000_000
        //     false
        // })));

        // rt.idle().await;

        Ok(Self {
            runtime: rt,
            scripts: HashMap::new(),
            runs: HashMap::new(),
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
    pub async fn extract_inputs(
        &self,
        runtime: &rquickjs::AsyncRuntime,
    ) -> Result<ScriptExtracted> {
        let context = rquickjs::AsyncContext::full(runtime).await.unwrap();

        let extracted = Arc::new(std::sync::Mutex::new(ScriptExtracted::default()));
        let extracted_for_ctx = extracted.clone();

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
        runtime: &rquickjs::AsyncRuntime,
        _input: ScriptInputData,
    ) -> Result<RunController> {
        let context = rquickjs::AsyncContext::full(runtime).await.unwrap();

        // TODO: handle http
        // tokio::spawn(async move {});

        async_with!(context => |ctx| {
            // SAFETY: the bytecode was compiled ourselves in `load_script`
            let raw_module = unsafe { rquickjs::Module::load(ctx.clone(), &self.bytecode).unwrap() };
            let (_module, _promise) = raw_module.eval().unwrap();

            // TODO: somehow run this js callback?
            // ctl.onButton(id, label, async ({ fs, net }) => {
            //   // do something
            // });

            // TODO: somehow implement ctl.onHttp?
            // - make `Run`s automatically go to sleep
            // - run a function/callback passed into js controller

            rquickjs::Result::Ok(())
        })
        .await
        .unwrap();

        Ok(RunController {
            run: Run {
                id: RunId::new(),
                script_id: self.script_id,
                created_at: Time::now_utc(),
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
            return Ok(Arc::clone(rt.value()));
        }

        let rt = Arc::new(ChannelRuntime::new().await?);

        // create a broadcast channel for script events on this channel
        let (tx, _) = broadcast::channel(100);
        self.script_event_txs.insert(channel_id, tx);
        self.runtimes.insert(channel_id, Arc::clone(&rt));
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
    // TODO: start async processing task to validate the script and transition to Valid/Invalid
    pub async fn create_script(&self, script: Script) -> Result<()> {
        self.process(script, None).await
        // TODO: insert script into database
    }

    /// create a script version
    pub async fn create_script_version(&self, script: Script, ver: ScriptVersion) -> Result<()> {
        self.process(script, Some(ver)).await
        // TODO: insert script version into database
    }

    /// process a script
    ///
    /// - does basic validation
    /// - extracts script inputs
    async fn process(&self, script: Script, ver: Option<ScriptVersion>) -> Result<()> {
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
        let source = str::from_utf8(&bytes).unwrap();

        let rt = self.init_rt(script.channel_id).await?;
        let loaded = rt.load_script(script.id, "strobbery", source).await?;
        let inputs = loaded.extract_inputs(&rt.runtime).await?;

        dbg!(inputs);

        Ok(())
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
        let ctl = script.spawn(&rt.runtime, input).await?;

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
