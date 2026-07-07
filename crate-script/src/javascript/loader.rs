use std::str::FromStr;

use common::v1::types::ids::RedexId;
use rquickjs::Module;

pub struct ModuleResolver;

pub struct ModuleLoader;

/// a reference to a module that can be loaded
#[derive(Debug)]
pub enum ModuleRef {
    /// load a builtin module: `lamprey:name`
    Builtin(BuiltinModule),

    /// load another script as a module: `script:uuid-here`
    Script(RedexId),
    // NOTE: maybe in the future i'll allow importing `https://path/to/somewhere`, `npm:foo`, `jsr:foo`?
}

#[derive(Debug)]
pub enum BuiltinModule {
    /// access to the lamprey api
    Api,

    /// configuration data (environment variables)
    Env,

    /// http types
    Http,

    /// network access
    Net,

    /// managing and communicating with other runs
    Run,

    /// persistent storage
    Storage,
}

impl ModuleResolver {
    pub fn new() -> Self {
        Self
    }
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self
    }
}

impl rquickjs::loader::Resolver for ModuleResolver {
    fn resolve<'js>(
        &mut self,
        ctx: &rquickjs::prelude::Ctx<'js>,
        base: &str,
        name: &str,
        attributes: Option<rquickjs::loader::ImportAttributes<'js>>,
    ) -> rquickjs::Result<String> {
        dbg!(base, name, attributes);
        Ok(name.to_string())
    }
}

impl rquickjs::loader::Loader for ModuleLoader {
    fn load<'js>(
        &mut self,
        ctx: &rquickjs::prelude::Ctx<'js>,
        name: &str,
        _attributes: Option<rquickjs::loader::ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js, rquickjs::module::Declared>> {
        let resolved: ModuleRef = name
            .parse()
            .map_err(|_| rquickjs::Error::new_loading(name))?;

        match resolved {
            ModuleRef::Builtin(b) => match b {
                BuiltinModule::Http => Module::declare_def::<super::glue::http::js_inner, _>(
                    ctx.clone(),
                    "lamprey:http",
                ),
                _ => Err(rquickjs::Error::new_loading(name)),
                // // these modules are pretty incomplete
                // BuiltinModule::Net => {
                //     Module::declare_def::<super::glue::net::js_inner, _>(ctx.clone(), "lamprey:net")
                // }
                // BuiltinModule::Run => {
                //     Module::declare_def::<super::glue::run::js_inner, _>(ctx.clone(), "lamprey:run")
                // }
                // BuiltinModule::Storage => Module::declare_def::<super::glue::storage::js_inner, _>(
                //     ctx.clone(),
                //     "lamprey:storage",
                // ),
                // BuiltinModule::Api => {
                //     Module::declare_def::<super::glue::api::js_inner, _>(ctx.clone(), "lamprey:api")
                // }
                // BuiltinModule::Env => {
                //     Module::declare_def::<super::glue::env::js_inner, _>(ctx.clone(), "lamprey:env")
                // }
            },
            ModuleRef::Script(_i) => {
                // TODO: somehow load this?
                // unsure how to load modules async (need to fetch from db)
                Err(rquickjs::Error::new_loading(name))
            }
        }
    }
}

// TODO: better errors
impl FromStr for BuiltinModule {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "api" => Ok(Self::Api),
            "env" => Ok(Self::Env),
            "http" => Ok(Self::Http),
            "net" => Ok(Self::Net),
            "run" => Ok(Self::Run),
            "storage" => Ok(Self::Storage),
            _ => Err(()),
        }
    }
}

// TODO: better errors
impl FromStr for ModuleRef {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.strip_prefix("lamprey:") {
            return BuiltinModule::from_str(s).map(ModuleRef::Builtin);
        }

        if let Some(s) = s.strip_prefix("script:") {
            return RedexId::from_str(s).map(ModuleRef::Script).map_err(|_| ());
        }

        Err(())
    }
}
