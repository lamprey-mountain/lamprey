use std::sync::Arc;

use async_trait::async_trait;
#[cfg(feature = "javascript")]
use common::v1::types::RedexVerId;
use common::v1::types::{
    redex::{metadata::RedexMetadata, Eval, EvalInput, EvalLogEntry, EvalStatus, RedexHandler},
    EvalId, RedexId,
};

#[cfg(feature = "wasm")]
use crate::wasm::WasmManager;

#[cfg(feature = "javascript")]
use crate::javascript::JsManager;

use crate::{Limits, Result};

/// an execution engine for arbitrary scripts
///
/// scripts run singlethreaded but there may be multiple scripts running in
/// parallel
pub struct Engine {
    limits: Limits,

    #[cfg(feature = "javascript")]
    js: JsManager,

    #[cfg(feature = "wasm")]
    wasm: WasmManager,
}

impl Engine {
    pub fn new(limits: Limits) -> Result<Self> {
        Ok(Self {
            limits: limits.clone(),

            #[cfg(feature = "javascript")]
            js: JsManager::new(limits.clone()),

            #[cfg(feature = "wasm")]
            wasm: WasmManager::new(limits.clone())?,
        })
    }

    #[cfg(feature = "javascript")]
    pub async fn load_js(
        &self,
        script_id: RedexId,
        script_version_id: RedexVerId,
        module_name: &str,
        module_source: &str,
    ) -> Result<Box<dyn Executor>> {
        let exec = self
            .js
            .load(script_id, script_version_id, module_name, module_source)
            .await?;
        Ok(Box::new(exec))
    }

    #[cfg(feature = "wasm")]
    pub async fn load_wasm(
        &self,
        script_id: RedexId,
        script_version_id: RedexVerId,
        module_name: &str,
        module_source: &str,
    ) -> Result<Box<dyn Executor>> {
        let exec = self
            .wasm
            .load(script_id, script_version_id, module_name, module_source)
            .await?;
        Ok(Box::new(exec))
    }

    /// get the configured limits of this engine
    pub fn limits(&self) -> &Limits {
        &self.limits
    }
}

/// a loaded script that is able to be run
#[async_trait]
pub trait Executor: Send + Sync {
    /// spawn this script
    async fn spawn(&self, input: EvalInput, run_id: EvalId) -> Result<Box<dyn ExecutionHandle>>;
}

/// a handle to a script running in an isolated context
#[async_trait]
pub trait ExecutionHandle: Send + Sync {
    /// get associated eval for this execution
    fn eval(&self) -> &Eval;

    /// get associated eval id for this execution
    fn eval_id(&self) -> EvalId;

    /// stop script execution
    fn stop(&self);

    // /// serialize this run to disk
    // fn save(&self);

    // /// deserialize this run from disk
    // fn restore() -> Self;

    /// poll for events
    async fn poll(&mut self) -> Result<Arc<ExecutionEvent>>;

    /// wait for this execution to finish, returning the extraction data if extraction was successful
    async fn done(&mut self) -> Result<ScriptExtracted>;

    // /// wait for this execution to finish, returning the returned http response if this used an http input
    // async fn done_http_response(&mut self) -> Result<()>;

    // HACK: get cloning to work
    fn clone_box(&self) -> Box<dyn ExecutionHandle>;
}

impl Clone for Box<dyn ExecutionHandle> {
    fn clone(&self) -> Box<dyn ExecutionHandle> {
        self.clone_box()
    }
}

#[derive(Debug, Clone)]
pub struct ScriptExtracted {
    pub metadata: RedexMetadata,
    pub inputs: Vec<RedexHandler>,
}

impl ScriptExtracted {
    pub fn new(name: String) -> Self {
        Self {
            metadata: RedexMetadata::new(name),
            inputs: vec![],
        }
    }
}

#[derive(Debug)]
pub enum ExecutionEvent {
    /// a log event was received
    Log(EvalLogEntry),

    /// run status changed
    Status(EvalStatus),

    /// script info has been extracted
    Extracted(ScriptExtracted),
    // /// metrics were received
    // Metrics,
    HttpResponse(http::Response<bytes::Bytes>),
}

pub type AnyExecutionHandle = Box<dyn ExecutionHandle>;
