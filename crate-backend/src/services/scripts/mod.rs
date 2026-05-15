use common::v1::types::redex::{
    Eval, EvalInput, EvalStatus, Redex, RedexFormat, RedexVersion, RedexVersionStatus,
};
use common::v1::types::util::Time;
use common::v1::types::{
    ChannelId, ConnectionId, EvalId, MediaId, MessageSync, RedexId, RedexVerId, UserId,
};
use dashmap::DashMap;
use lamprey_script::engine::{AnyExecutionHandle, ExecutionEvent, ScriptExtracted};
use lamprey_script::{Engine, Executor, Limits};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::error::Result;
use crate::{Error, ServerStateInner};

/// the service that manages all scripts
pub struct ServiceScripts {
    state: Arc<ServerStateInner>,

    engine: Engine,
    handles: DashMap<EvalId, AnyExecutionHandle>,

    /// broadcast channels for script events per channel_id
    script_event_txs: DashMap<ChannelId, broadcast::Sender<MessageSync>>,
}

impl ServiceScripts {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            engine: Engine::new(Limits::strict()).unwrap(),
            handles: DashMap::new(),
            script_event_txs: DashMap::new(),
        }
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

    /// get the redex version id for a redex
    pub async fn get_redex_version_id(&self, redex_id: RedexId) -> Result<RedexVerId> {
        let script = self
            .state
            .data()
            .script_get(redex_id)
            .await?
            .ok_or(Error::BadStatic("script not found"))?;
        Ok(script.latest_version.version_id)
    }

    /// get or create a broadcast receiver for script events on a channel
    pub async fn subscribe_channel(
        &self,
        channel_id: ChannelId,
    ) -> Result<broadcast::Receiver<MessageSync>> {
        if let Some(entry) = self.script_event_txs.get(&channel_id) {
            Ok(entry.value().subscribe())
        } else {
            let (tx, rx) = broadcast::channel(100);
            self.script_event_txs.insert(channel_id, tx);
            Ok(rx)
        }
    }

    async fn load_from_source(
        &self,
        redex_id: RedexId,
        redex_version_id: RedexVerId,
        media_id: MediaId,
        format: RedexFormat,
    ) -> Result<Box<dyn Executor>> {
        let bytes = self.state.services().media.download(media_id).await?;
        let source = str::from_utf8(&bytes).unwrap();
        let loaded = match format {
            RedexFormat::Javascript => self
                .engine
                .load_js(redex_id, redex_version_id, "strobbery", source)
                .await
                .unwrap(),
            RedexFormat::Webassembly => self
                .engine
                .load_wasm(redex_id, redex_version_id, "strobbery", source)
                .await
                .unwrap(),
        };
        Ok(loaded)
    }

    /// create a script
    // TODO: process script (and script version) in background
    pub async fn create_script(&self, script: Redex) -> Result<()> {
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
        data.script_version_update_status(script.id, version_id, RedexVersionStatus::Valid)
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
    pub async fn create_script_version(&self, script: Redex, ver: RedexVersion) -> Result<()> {
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
        data.script_version_update_status(script.id, version_id, RedexVersionStatus::Valid)
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
                    redex_id: script.id,
                    version: full_ver,
                },
            )
            .await;
        }

        Ok(())
    }

    /// create a new script syncer for a session
    pub fn create_syncer(&self, conn_id: ConnectionId) -> ScriptSyncer {
        ScriptSyncer::new(Arc::clone(&self.state), conn_id)
    }

    /// load a redex
    async fn load(&self, redex_id: RedexId) -> Result<Box<dyn Executor>> {
        // TODO: check if script is already loaded first
        // self.engine.get_js(&script_id);

        let srv = self.state.services();
        let mut data = self.state.data();

        let script = data
            .script_get(redex_id)
            .await?
            .ok_or(Error::BadStatic("script not found"))?;
        dbg!(&script.status);
        // TODO: verify the script status is Valid? for `spawn` but not `process`.

        let bytes = srv
            .media
            .download(script.latest_version.location.media_id().unwrap())
            .await?;
        let source = str::from_utf8(&bytes).unwrap();

        // TODO: module name
        let loaded = self
            .engine
            .load_js(
                redex_id,
                script.latest_version.version_id,
                "strobbery",
                source,
            )
            .await
            .unwrap();

        Ok(loaded)
    }

    /// process a script
    ///
    /// - does basic validation
    /// - extracts script inputs and metadata
    /// - optionally process a specific version of a script
    async fn process(&self, script: Redex, ver: Option<RedexVersion>) -> Result<ScriptExtracted> {
        // NOTE: should i insert the extraction run in the db too?

        let latest_version = ver.as_ref().unwrap_or(&script.latest_version);
        let version_id = latest_version.version_id;
        let location = &latest_version.location;
        let format = &latest_version.format;
        let media_id = location.media_id().unwrap();
        let loaded = self
            .load_from_source(script.id, version_id, media_id, format.clone())
            .await?;

        let mut handle = loaded
            .spawn(EvalInput::Extraction, EvalId::new())
            .await
            .unwrap();
        let extracted = handle.done().await.unwrap();

        Ok(extracted)
    }

    /// spawn a script
    pub async fn spawn(
        &self,
        channel_id: ChannelId,
        redex_id: RedexId,
        redex_version_id: RedexVerId,
        input: EvalInput,
    ) -> Result<AnyExecutionHandle> {
        // load redex
        let loaded = self.load(redex_id).await?;
        let eval_id = EvalId::new();

        // insert run into database
        let run = Eval {
            id: eval_id,
            redex_id,
            redex_version_id,
            created_at: Time::now_utc(),
            stopped_at: None,
            status: EvalStatus::Creating,
            input: input.clone().into(),
        };
        let mut data = self.state.data();
        data.script_run_create(&run).await?;
        self.broadcast(
            channel_id,
            MessageSync::ScriptRunCreate {
                channel_id,
                run: run.clone(),
            },
        )
        .await;

        let handle = loaded.spawn(input, eval_id).await.unwrap();
        self.handles.insert(eval_id, handle.clone());
        let caller_handle = handle.clone();
        let mut event_handle = handle; // move the original receiver so we don't miss any messages
        let state = self.state.clone();

        // handle execution events, propagate them to api sync events
        tokio::spawn(async move {
            while let Ok(event) = event_handle.poll().await {
                match &*event {
                    ExecutionEvent::Log(entry) => {
                        let mut data = state.data();
                        let _ = data.script_log_insert(eval_id, entry).await;
                        state
                            .services()
                            .scripts
                            .broadcast(
                                channel_id,
                                MessageSync::ScriptLogCreate {
                                    channel_id,
                                    run_id: eval_id,
                                    entry: entry.clone(),
                                },
                            )
                            .await;
                    }
                    ExecutionEvent::Status(status) => {
                        let mut data = state.data();
                        let _ = data.script_run_update_status(eval_id, status.clone()).await;

                        let run_info = event_handle.eval();
                        let stopped_at = if matches!(
                            *status,
                            EvalStatus::Exited | EvalStatus::Crashed | EvalStatus::Stopped
                        ) {
                            Some(Time::now_utc())
                        } else {
                            None
                        };

                        state
                            .services()
                            .scripts
                            .broadcast(
                                channel_id,
                                MessageSync::ScriptRunUpdate {
                                    channel_id,
                                    run: Eval {
                                        id: eval_id,
                                        redex_id,
                                        redex_version_id,
                                        created_at: run_info.created_at,
                                        stopped_at,
                                        status: status.clone(),
                                        input: run_info.input.clone(),
                                    },
                                },
                            )
                            .await;

                        if stopped_at.is_some() {
                            break;
                        }
                    }
                    ExecutionEvent::Extracted(_) => {}
                    ExecutionEvent::HttpResponse(_) => {}
                }
            }

            // cleanup
            state.services().scripts.handles.remove(&eval_id);
        });

        Ok(caller_handle)
    }

    /// stop a script run
    pub async fn stop_run(
        &self,
        channel_id: ChannelId,
        _script_id: RedexId,
        run_id: EvalId,
    ) -> Result<()> {
        let handle = self.handles.get(&run_id).ok_or(Error::NotFound)?;
        handle.stop();

        let mut data = self.state.data();
        let _ = data
            .script_run_update_status(run_id, EvalStatus::Stopped)
            .await;

        self.broadcast(
            channel_id,
            MessageSync::ScriptRunUpdate {
                channel_id,
                run: handle.eval().to_owned(),
            },
        )
        .await;

        Ok(())
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
    query_tx: tokio::sync::watch::Sender<Option<(ChannelId, RedexId)>>,

    /// Receives subscription requests from `query_tx`. The poll() loop monitors
    /// this receiver for changes. When a new query arrives, it sets up a
    /// subscription to the requested script and moves the subscription to `current_rx`.
    query_rx: tokio::sync::watch::Receiver<Option<(ChannelId, RedexId)>>,

    /// The active script subscription. Contains the current (channel_id, script_id) tuple
    /// and a broadcast receiver for receiving script events (logs, metrics, runs).
    current_rx: Option<((ChannelId, RedexId), broadcast::Receiver<MessageSync>)>,

    /// The connection ID associated with this syncer, used to filter out
    /// self-originated events.
    conn_id: ConnectionId,

    /// Queue of pending sync messages to be sent to the client.
    pending_sync: VecDeque<MessageSync>,

    /// The user ID of the authenticated user.
    user_id: Option<UserId>,
}

impl ScriptSyncer {
    /// create a new syncer
    pub(super) fn new(s: Arc<ServerStateInner>, conn_id: ConnectionId) -> Self {
        let (query_tx, query_rx) = tokio::sync::watch::channel(None);
        Self {
            s,
            query_tx,
            query_rx,
            current_rx: None,
            conn_id,
            pending_sync: VecDeque::new(),
            user_id: None,
        }
    }

    pub async fn set_user_id(&mut self, user_id: Option<UserId>) {
        self.user_id = user_id;
    }

    /// Set the script to subscribe to.
    pub async fn set_context_id(&self, channel_id: ChannelId, script_id: RedexId) -> Result<()> {
        self.query_tx
            .send(Some((channel_id, script_id)))
            .map_err(|_| Error::Internal("query channel closed".to_string()))?;
        Ok(())
    }

    /// Check if client is actively subscribed to a script.
    pub fn is_subscribed(&self, channel_id: &ChannelId, script_id: &RedexId) -> bool {
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
                            redex_id: script_id,
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
