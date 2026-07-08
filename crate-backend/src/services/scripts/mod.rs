use common::v1::types::redex::{
    Eval, EvalInput, EvalStatus, Redex, RedexFormat, RedexVersion, RedexVersionStatus,
};
use common::v1::types::util::Time;
use common::v1::types::{
    ChannelId, ConnectionId, EvalId, MediaId, MessageSync, RedexId, RedexVerId,
};
use dashmap::DashMap;
use lamprey_script::engine::{AnyExecutionHandle, ExecutionEvent, ScriptExtracted};
use lamprey_script::{Engine, Executor, Limits};
use tokio::sync::broadcast;

use crate::prelude::*;
use crate::services::scripts::sync::ScriptSyncer;

mod sync;

/// the service that manages all scripts
pub struct ServiceScripts {
    globals: Globals,

    engine: Engine,
    handles: DashMap<EvalId, AnyExecutionHandle>,

    /// broadcast channels for script events per channel_id
    script_event_txs: DashMap<ChannelId, broadcast::Sender<MessageSync>>,
}

impl ServiceScripts {
    pub fn new(globals: Globals) -> Self {
        Self {
            globals,
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
            .globals
            .services()
            .channels
            .get(channel_id, None)
            .await
            .ok();
        if let Some(room_id) = chan.and_then(|c| c.room_id) {
            let _ = self.globals.messaging().broadcast_room(room_id, msg).await;
        }
    }

    /// get the redex version id for a redex
    pub async fn get_redex_version_id(&self, redex_id: RedexId) -> Result<RedexVerId> {
        let script = self
            .globals
            .begin_read()
            .await?
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
        let item = self.globals.services().media.get(media_id).await?;
        let bytes = item.download_bytes().await?;
        let loaded = match format {
            RedexFormat::Javascript => {
                let source = std::str::from_utf8(&bytes)?;
                self.engine
                    .load_js(redex_id, redex_version_id, "strobbery", source)
                    .await?
            }
            RedexFormat::Webassembly => {
                self.engine
                    .load_wasm(redex_id, redex_version_id, "strobbery", &bytes)
                    .await?
            }
        };
        Ok(loaded)
    }

    /// create a script
    // TODO: process script (and script version) in background
    pub async fn create_script(&self, script: Redex) -> Result<()> {
        let inputs = self.process(script.clone(), None).await?;
        let extracted_metadata = inputs.metadata;
        let mut data = self.globals.begin().await?;

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

        data.commit().await?;

        Ok(())
    }

    /// create a script version
    pub async fn create_script_version(&self, script: Redex, ver: RedexVersion) -> Result<()> {
        let ver_format = ver.format.clone();
        let ver_location = ver.location.clone();
        let ver_metadata = ver.metadata.clone();
        let inputs = self.process(script.clone(), Some(ver)).await?;

        let mut data = self.globals.begin().await?;

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

        data.commit().await?;

        Ok(())
    }

    /// create a new script syncer for a session
    pub fn create_syncer(&self, conn_id: ConnectionId) -> ScriptSyncer {
        ScriptSyncer::new(self.globals.clone(), conn_id)
    }

    /// load a redex
    async fn load(&self, redex_id: RedexId) -> Result<Box<dyn Executor>> {
        // TODO: check if script is already loaded first
        // self.engine.get_js(&script_id);

        let srv = self.globals.services();
        let mut data = self.globals.begin_read().await?;

        let script = data
            .script_get(redex_id)
            .await?
            .ok_or(Error::BadStatic("script not found"))?;
        // TODO: verify the script status is Valid? for `spawn` but not `process`.

        let media_id = script.latest_version.location.media_id().unwrap();
        let item = srv.media.get(media_id).await?;
        let bytes = item.download_bytes().await?;

        let loaded = match script.latest_version.format {
            RedexFormat::Javascript => {
                let source = std::str::from_utf8(&bytes)?;
                self.engine
                    .load_js(
                        redex_id,
                        script.latest_version.version_id,
                        "strobbery",
                        source,
                    )
                    .await
                    .unwrap()
            }
            RedexFormat::Webassembly => self
                .engine
                .load_wasm(
                    redex_id,
                    script.latest_version.version_id,
                    "strobbery",
                    &bytes,
                )
                .await
                .unwrap(),
        };

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
        let mut data = self.globals.begin().await?;
        data.script_run_create(&run).await?;
        data.commit().await?;

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
        let state = self.globals.clone();

        // handle execution events, propagate them to api sync events
        tokio::spawn(async move {
            while let Ok(event) = event_handle.poll().await {
                match &*event {
                    ExecutionEvent::Log(entry) => {
                        if let Ok(mut data) = state.begin().await {
                            let _ = data.script_log_insert(eval_id, entry).await;
                            let _ = data.commit().await;
                        }
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
                        if let Ok(mut data) = state.begin().await {
                            let _ = data.script_run_update_status(eval_id, status.clone()).await;
                            let _ = data.commit().await;
                        }

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

        let mut data = self.globals.begin().await?;
        let _ = data
            .script_run_update_status(run_id, EvalStatus::Stopped)
            .await;
        data.commit().await?;

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
