//! generated bindings

use std::sync::Arc;

use common::v1::types::{
    metadata::Metadata,
    redex::{EvalLogEntry, EvalLogLevel, EvalLogSource},
    util::Time,
};
use wasmtime::component::bindgen;

use crate::{engine::ExecutionEvent, wasm::wit::lamprey::scripting::log::Level};

use super::WasmState;

bindgen!({
    imports: { default: trappable },
});

impl lamprey::scripting::types::Host for WasmState {}
impl lamprey::scripting::network::Host for WasmState {}

impl lamprey::scripting::log::Host for WasmState {
    fn log(
        &mut self,
        level: Level,
        content: String,
        attrs: Vec<(String, String)>,
    ) -> wasmtime::Result<()> {
        let level = match level {
            Level::Debug => EvalLogLevel::Debug,
            Level::Info => EvalLogLevel::Info,
            Level::Warning => EvalLogLevel::Warning,
            Level::Error => EvalLogLevel::Error,
        };

        let entry = EvalLogEntry {
            id: 0,
            created_at: Time::now_utc(),
            level,
            source: EvalLogSource::Redex {
                redex_id: self.redex_id,
                trace_id: None,
                target: None,
                line: None,
                column: None,
            },
            content,
            attributes: Metadata(attrs.into_iter().collect()),
        };

        let _ = self.sender.send(Arc::new(ExecutionEvent::Log(entry)));

        Ok(())
    }
}
