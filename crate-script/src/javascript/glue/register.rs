use nanoid::nanoid;
use rquickjs::{
    class::{Trace, Tracer},
    Ctx, Function, JsLifetime, Object, Persistent, Result as JsResult,
};

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
#[derive(Clone, Trace, JsLifetime)]
pub struct ScriptRegister {}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl ScriptRegister {
    /// create a new input that runs when manually triggered
    #[qjs(rename = "onTrigger")]
    fn on_trigger(&self) -> InputBuilder {
        let alphabet: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
            .chars()
            .collect();

        InputBuilder {
            id: nanoid!(8, &alphabet),
            label: None,
            permissions: vec![],
            callback: None,
        }
    }

    /// create a new input that runs when a http request is received
    fn on_http(&self) -> InputBuilder {
        todo!()
    }

    /// create a new input that runs every once in a while
    fn on_cron(&self) -> InputBuilder {
        todo!()
    }

    fn on_spawn(&self) -> InputBuilder {
        todo!()
    }

    fn on_message(&self) -> InputBuilder {
        todo!()
    }

    /// create a new input that runs when an api event is received
    fn on_event(&self) -> InputBuilder {
        todo!()
    }
}

#[rquickjs::class]
#[derive(Clone, JsLifetime)]
pub struct InputBuilder {
    pub id: String,
    pub label: Option<String>,
    pub permissions: Vec<String>,
    pub callback: Option<Persistent<Function<'static>>>,
}

// manually implement Trace because Persistent doesn't implement it
// since Persistent is a root, we don't need to visit it during tracing
impl<'js> Trace<'js> for InputBuilder {
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

    fn run<'js>(mut self, cx: Ctx<'js>, cb: Function<'js>) -> JsResult<()> {
        self.label.get_or_insert_with(|| self.id.clone());

        let cb_static = Persistent::save(&cx, cb);
        self.callback = Some(cb_static);

        let registry = cx.globals().get::<_, Object>("__registry")?;
        let callbacks: Object = registry.get("callbacks")?;
        callbacks.set(self.id.clone(), self.callback.as_ref().unwrap().clone())?;

        // Also store input metadata for extract
        let inputs: rquickjs::Array = registry.get("inputs")?;
        let input_obj = Object::new(cx.clone())?;
        input_obj.set("id", self.id.clone())?;
        input_obj.set("label", self.label.clone())?;
        input_obj.set("type", "manual")?;
        inputs.set(inputs.len(), input_obj)?;

        Ok(())
    }
}
