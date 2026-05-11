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

pub enum BuiltinModule {
    Http,
    Net,
    Run,
    Storage,
    // etc...
}

impl rquickjs::loader::Loader for ModuleLoader {
    fn load<'js>(
        &mut self,
        _ctx: &rquickjs::Ctx<'js>,
        name: &str,
    ) -> rquickjs::Result<rquickjs::Module<'js, rquickjs::module::Declared>> {
        let resolved: ModuleRef = name
            .parse()
            .map_err(|_| rquickjs::Error::new_loading(name))?;

        match resolved {
            ModuleRef::Builtin(_b) => todo!("load builtin module"),
            ModuleRef::Script(_i) => todo!("somehow load this?"),
            // unsure how to load modules async (need to fetch from db)
        }
    }
}

impl FromStr for BuiltinModule {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lamprey:http" => Ok(Self::Http),
            "lamprey:net" => Ok(Self::Net),
            "lamprey:run" => Ok(Self::Run),
            "lamprey:storage" => Ok(Self::Storage),
            _ => Err(()),
        }
    }
}

impl FromStr for ModuleRef {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.strip_prefix("lamprey:") {
            return BuiltinModule::from_str(s).map(ModuleRef::Builtin);
        }

        if let Some(s) = s.strip_prefix("script:") {
            return ScriptId::from_str(s).map(ModuleRef::Script).map_err(|_| ());
        }

        Err(())
    }
}
