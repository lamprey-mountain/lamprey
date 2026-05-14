// script uses?
// - unfurling
// - custom room logic (eg. permissions)
// - hosted bots
// - ~~maybe custom server stuff like cloudflare workers?~~
pub trait ScriptPurpose {
    /// the name of this purpose
    fn name(&self) -> String;

    /// the capabilities that can be used by this script
    fn capabilities(&self) -> ScriptCapabilities;

    /// the exports this needs
    fn needs(&self) -> ScriptNeeds;
}

pub struct ScriptCapabilities;

pub struct ScriptNeeds;
