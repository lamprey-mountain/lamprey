/// what a script can be written to do
pub trait ScriptPurpose {
    /// the name of this purpose
    fn name(&self) -> String;

    /// the capabilities that scripts for this purpose can use
    fn capabilities(&self) -> ScriptCapabilities;

    /// the exports scripts need to provide
    fn needs(&self) -> ScriptNeeds;
}

// script uses?
// - unfurling
// - custom room logic (eg. permissions)
// - hosted bots
// - also include some kind of "test" purpose

pub struct ScriptCapabilities {
    // TODO
    // do i have one giant list of capabilities then have this filter them?
    // what if some purpose has a custom set of capabilities?
}

pub struct ScriptNeeds {
    // TODO
    // maybe reuse utoipa ToSchema to generate types for these
}
