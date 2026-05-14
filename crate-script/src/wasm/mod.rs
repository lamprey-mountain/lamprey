use std::sync::Arc;

use crate::{
    engine::{ExecutionEvent, ScriptExtracted},
    Error, ExecutionHandle, Executor, Limits, Result,
};
use async_trait::async_trait;
use common::v1::types::{
    redex::{
        metadata::{License, RedexMetadata, Semver},
        Eval, EvalInput, EvalStatus,
    },
    util::Time,
    EvalId, RedexId, RedexVerId,
};
use tokio::sync::{broadcast, watch};
use wasmtime::{
    component::{Component, HasSelf, Linker, ResourceTable},
    Config, Engine, Store,
};

mod glue;
mod wit;

pub struct WasmManager {
    limits: Limits,
    engine: Engine,
}

/// host-specific wasm state
struct WasmState {
    table: ResourceTable,
    redex_id: RedexId,
    sender: broadcast::Sender<Arc<ExecutionEvent>>,
}

/// executes a wasm script
pub struct WasmExecutor {
    engine: Engine,
    component: Component,
    linker: Arc<Linker<WasmState>>,
    redex_id: RedexId,
    redex_version_id: RedexVerId,
    limits: Limits,
    // script: Arc<JsCompiledScript>,
}

/// a handle to a live wasm execution
pub struct WasmHandle {
    run: Arc<Eval>,
    // stop_signal: Arc<AtomicBool>,
    events: broadcast::Receiver<Arc<ExecutionEvent>>,
    ext_recv: watch::Receiver<Option<ScriptExtracted>>,
}

/// a compiled script loaded in memory
struct WasmCompiledScript {
    // script_id: ScriptId,
    // bytecode: Vec<u8>,
}

impl WasmManager {
    pub fn new(limits: Limits) -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.wasm_component_model_async(true);
        config.epoch_interruption(true);

        // config.cache(cache)
        // let cache = Cache::new(CacheConfig::new().with_directory(directory))

        let engine = Engine::new(&config)?;
        Ok(Self { limits, engine })
    }

    /// load a wasm script
    pub async fn load(
        &self,
        redex_id: RedexId,
        redex_version_id: RedexVerId,
        _module_name: &str,
        module_source: &str,
    ) -> Result<WasmExecutor> {
        let component = Component::new(&self.engine, module_source)?;
        let mut linker = Linker::new(&self.engine);

        wit::ScriptWorld::add_to_linker::<_, HasSelf<_>>(&mut linker, |state: &mut WasmState| {
            state
        })?;

        Ok(WasmExecutor {
            engine: self.engine.clone(),
            component,
            linker: Arc::new(linker),
            redex_id,
            redex_version_id,
            limits: self.limits.clone(),
        })
    }
}

#[async_trait]
impl Executor for WasmExecutor {
    /// spawn this script
    async fn spawn(&self, input: EvalInput, eval_id: EvalId) -> Result<Box<dyn ExecutionHandle>> {
        let (events_tx, events_rx) = broadcast::channel(100);
        let (ext_tx, ext_rx) = watch::channel(None);

        let redex_id = self.redex_id;
        let redex_version_id = self.redex_version_id;
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

        let state = WasmState {
            table: ResourceTable::new(),
            sender: events_tx.clone(),
            redex_id,
        };
        let mut store = Store::new(&self.engine, state);
        store.set_epoch_deadline(1); // TODO: link to your Limits system

        let linker = self.linker.clone();
        let component = self.component.clone();

        tokio::spawn(async move {
            let _ = events_tx.send(Arc::new(ExecutionEvent::Status(EvalStatus::Active)));

            let result: wasmtime::Result<()> = async {
                let bindings =
                    wit::ScriptWorld::instantiate_async(&mut store, &component, &linker).await?;

                let metadata = bindings.call_get_metadata(&mut store)?;
                let extracted = ScriptExtracted {
                    metadata: RedexMetadata {
                        name: metadata.name,
                        description: metadata.description,
                        version: metadata.version.map(Semver),
                        license: metadata.license.map(License),
                        // TODO
                        homepage_url: None,
                        authors: vec![],
                        origin: None,
                    },
                    inputs: vec![], // TODO: populate
                };

                let _ = ext_tx.send(Some(extracted));

                // // Start a thread that will bump the epoch after 1 second.
                // let engine_clone = engine.clone();
                // std::thread::spawn(move || {
                //     std::thread::sleep(std::time::Duration::from_secs(1));
                //     engine_clone.increment_epoch();
                // });

                match input {
                    EvalInput::Extraction => {
                        // no special stuff needed here
                    }
                    EvalInput::Http { request } => {
                        let res =
                            bindings.call_handle_http(&mut store, "no_id?", &request.into())?;
                        let _ = events_tx.send(Arc::new(ExecutionEvent::HttpResponse(res.into())));
                    }
                    EvalInput::Manual { id, .. } => {
                        bindings.call_handle_trigger(&mut store, &id)?;
                    }
                    EvalInput::Event { .. } => {
                        return Err(wasmtime::Error::msg("not yet implemented"));
                    }
                }

                Ok(())
            }
            .await;

            let final_status = if result.is_ok() {
                EvalStatus::Exited
            } else {
                EvalStatus::Crashed
            };

            let _ = events_tx.send(Arc::new(ExecutionEvent::Status(final_status)));
        });

        Ok(Box::new(WasmHandle {
            run,
            events: events_rx,
            ext_recv: ext_rx,
        }))
    }
}

#[async_trait]
impl ExecutionHandle for WasmHandle {
    fn eval(&self) -> &Eval {
        &*self.run
    }

    fn eval_id(&self) -> EvalId {
        self.run.id
    }

    fn stop(&self) {
        todo!()
    }

    async fn poll(&mut self) -> Result<Arc<ExecutionEvent>> {
        self.events
            .recv()
            .await
            .map_err(|e| Error::BroadcastRecv(e.to_string()))
    }

    async fn done(&mut self) -> Result<ScriptExtracted> {
        if let Some(ext) = self.ext_recv.borrow().clone() {
            return Ok(ext);
        }

        self.ext_recv
            .changed()
            .await
            .map_err(|e| Error::WatchChanged(e.to_string()))?;
        self.ext_recv
            .borrow()
            .clone()
            .ok_or(Error::ExtractionDataMissing)
    }

    fn clone_box(&self) -> Box<dyn ExecutionHandle> {
        Box::new(WasmHandle {
            run: self.run.clone(),
            events: self.events.resubscribe(),
            ext_recv: self.ext_recv.clone(),
        })
    }
}
