use common::v1::types::script::{ScriptFormat, ScriptLocation, ScriptMetadata};

#[derive(Debug, Clone)]
pub struct DataScriptVersion {
    pub format: ScriptFormat,
    pub location: ScriptLocation,
    pub metadata: ScriptMetadata,
}
