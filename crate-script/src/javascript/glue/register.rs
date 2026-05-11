use common::v1::types::script::{ScriptInput, ScriptInputType};
use nanoid::nanoid;
use rquickjs::{
    class::{Trace, Tracer},
    Ctx, Function, JsLifetime, Persistent, Result as JsResult,
};
use std::sync::{Arc, Mutex};

/// lets scripts register inputs and stuff
///
/// ## basic inputs
///
/// - `trigger`: manually ran
/// - `cron`: runs automatically on a timer
/// - `http`: runs when an http request comes in
///
/// ## api inputs
///
/// - `event`: when an api event is received
/// - todo: `automod`: when something needs to be checked by automod
/// - todo: `unfurl`: when a url needs to be unfurled (may be merged with interaction)
/// - todo: `interaction`: when an interaction is received
///
/// ## script inputs
///
/// - `spawn`: when a new run is spawned for this script
/// - `message`: when a message is sent to this script
#[rquickjs::class]
#[derive(Clone, JsLifetime)]
pub struct ScriptRegister {
    pub registry: Arc<Mutex<ScriptRegistry>>,
}

impl ScriptRegister {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Mutex::new(ScriptRegistry::new())),
        }
    }
}

impl ScriptRegistry {
    pub fn new() -> Self {
        Self { inputs: vec![] }
    }
}

impl ScriptRegister {
    fn input(&self, ty: ScriptInputType) -> InputBuilder {
        let alphabet: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
            .chars()
            .collect();

        InputBuilder {
            id: nanoid!(8, &alphabet),
            label: None,
            permissions: vec![],
            ty,
            registry: Arc::clone(&self.registry),
        }
    }
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl ScriptRegister {
    /// create a new input that runs when manually triggered
    fn on_trigger(&self) -> InputBuilder {
        self.input(ScriptInputType::Manual)
    }

    /// create a new input that runs when a http request is received
    fn on_http(&self) -> InputBuilder {
        self.input(ScriptInputType::Http {})
    }

    // /// create a new input that runs every once in a while
    // fn on_cron(&self) -> InputBuilder {
    //     todo!()
    // }

    // fn on_spawn(&self) -> InputBuilder {
    //     todo!()
    // }

    // fn on_message(&self) -> InputBuilder {
    //     todo!()
    // }

    // /// create a new input that runs when an api event is received
    // fn on_event(&self) -> InputBuilder {
    //     todo!()
    // }
}

#[rquickjs::class]
#[derive(Clone, JsLifetime)]
pub struct InputBuilder {
    pub id: String,
    pub label: Option<String>,
    pub permissions: Vec<String>,
    pub ty: ScriptInputType,
    pub registry: Arc<Mutex<ScriptRegistry>>,
}

// manually implement Trace because Persistent doesn't implement it
// since Persistent is a root, we don't need to visit it during tracing
impl<'js> Trace<'js> for InputBuilder {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

impl<'js> Trace<'js> for ScriptRegister {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl InputBuilder {
    fn needs(mut self, perms: Vec<String>) -> Self {
        self.permissions.extend(perms);
        self
    }

    fn id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    fn label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    fn run<'js>(mut self, ctx: Ctx<'js>, cb: Function<'js>) -> JsResult<()> {
        let label = self.label.get_or_insert_with(|| self.id.clone());

        let callback = Persistent::save(&ctx, cb);

        let mut reg = self.registry.lock().unwrap();
        reg.inputs.push(ScriptInputCallback {
            definition: ScriptInput {
                id: self.id,
                label: label.to_owned(),
                ty: self.ty,
                effects: vec![], // TODO
            },
            callback,
        });

        Ok(())
    }
}

#[derive(Debug)]
pub struct ScriptInputCallback {
    pub callback: Persistent<Function<'static>>,
    pub definition: ScriptInput,
}

pub struct ScriptRegistry {
    pub inputs: Vec<ScriptInputCallback>,
}

impl ScriptRegistry {
    /// get a script input by id
    pub fn find(&self, id: &str) -> Option<&ScriptInputCallback> {
        self.inputs.iter().find(|i| i.definition.id == id)
    }
}
