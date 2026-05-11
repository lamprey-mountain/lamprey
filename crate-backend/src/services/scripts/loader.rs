use std::str::FromStr;

use common::v1::types::ids::ScriptId;

pub struct ModuleLoader {
    // TODO
}

pub enum ModuleRef {
    /// load a builtin module: `lamprey:name`
    Builtin(BuiltinModule),

    /// load another script as a module: `script:uuid-here`
    Script(ScriptId),
    // NOTE: maybe in the future i'll allow importing `https://path/to/somewhere`, `npm:foo`, `jsr:foo`?
}

enum BuiltinModule {
    Http,
    Net,
    Run,
    Storage,
    // etc...
}

impl rquickjs::loader::Loader for ModuleLoader {
    fn load<'js>(
        &mut self,
        ctx: &rquickjs::Ctx<'js>,
        name: &str,
    ) -> rquickjs::Result<rquickjs::Module<'js, rquickjs::module::Declared>> {
        // parse name, load module
        todo!()
    }
}

impl FromStr for BuiltinModule {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl FromStr for ModuleRef {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}
