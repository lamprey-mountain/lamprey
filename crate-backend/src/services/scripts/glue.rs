use rquickjs::{
    class::{Trace, Tracer},
    Ctx, Function, JsLifetime, Object, Persistent, Result as JsResult,
};

/// lets scripts register inputs and stuff
#[rquickjs::class]
#[derive(Clone, Trace, JsLifetime)]
pub struct ScriptRegister {}

#[rquickjs::methods]
impl ScriptRegister {
    #[qjs(rename = "onTrigger")]
    fn on_trigger(&self) -> InputBuilder {
        InputBuilder {
            id: "default_id".to_string(), // TODO: use alphanumeric nanoid
            label: "Default Label".to_string(),
            permissions: vec![],
            callback: None,
        }
    }
}

#[rquickjs::class]
#[derive(Clone, JsLifetime)]
pub struct InputBuilder {
    pub id: String,
    pub label: String, // TODO: make Option
    pub permissions: Vec<String>,
    pub callback: Option<Persistent<Function<'static>>>,
}

// manually implement Trace because Persistent doesn't implement it
// since Persistent is a root, we don't need to visit it during tracing
impl<'js> Trace<'js> for InputBuilder {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

#[rquickjs::methods]
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
        self.label = label;
        self
    }

    // FIX: Use a named lifetime 'js to unify the Ctx and Function lifetimes
    fn run<'js>(mut self, cx: Ctx<'js>, cb: Function<'js>) -> JsResult<()> {
        // Now 'cx' and 'cb' are guaranteed to have the same lifetime
        let cb_static = Persistent::save(&cx, cb);
        self.callback = Some(cb_static);

        // Push this builder into a registry
        let registry = cx.globals().get::<_, Object>("__registry")?;
        let callbacks: Object = registry.get("callbacks")?;
        callbacks.set(self.id.clone(), self.callback.as_ref().unwrap().clone())?;

        Ok(())
    }
}
