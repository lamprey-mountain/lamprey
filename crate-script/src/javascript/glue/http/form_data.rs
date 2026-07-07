use rquickjs::{
    Ctx, JsLifetime, Result as JsResult,
    class::{Trace, Tracer},
};

#[derive(Clone, Default, JsLifetime)]
#[rquickjs::class]
pub struct FormData {
    pub(crate) fields: Vec<(String, String)>,
}

impl<'js> Trace<'js> for FormData {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl FormData {
    #[qjs(constructor)]
    fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// append a value to a field
    fn append(&mut self, name: String, value: String) {
        self.fields.push((name, value));
    }

    /// remove a field
    fn delete(&mut self, name: String) {
        self.fields.retain(|(n, _)| n != &name);
    }

    /// get a field value
    fn get(&self, name: String) -> Option<String> {
        self.fields
            .iter()
            .find(|(n, _)| n == &name)
            .map(|(_, v)| v.clone())
    }

    /// get all field values
    fn get_all(&self, name: String) -> Vec<String> {
        self.fields
            .iter()
            .filter(|(n, _)| n == &name)
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// check if a field exists
    fn has(&self, name: String) -> bool {
        self.fields.iter().any(|(n, _)| n == &name)
    }

    /// set a field value
    fn set(&mut self, name: String, value: String) {
        let mut found = false;
        for (n, v) in self.fields.iter_mut() {
            if n == &name {
                *v = value.clone();
                found = true;
            }
        }
        if !found {
            self.fields.push((name, value));
        }
    }

    /// iterate over all field entries
    fn entries<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Value<'js>> {
        let array = rquickjs::Array::new(ctx.clone())?;
        for (i, (name, value)) in self.fields.iter().enumerate() {
            let entry = rquickjs::Array::new(ctx.clone())?;
            entry.set(0, name.as_str())?;
            entry.set(1, value.as_str())?;
            array.set(i, entry)?;
        }
        Ok(array.into_value())
    }

    /// iterate over all field keys
    fn keys<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Value<'js>> {
        let array = rquickjs::Array::new(ctx.clone())?;
        for (i, (name, _)) in self.fields.iter().enumerate() {
            array.set(i, name.as_str())?;
        }
        Ok(array.into_value())
    }

    /// iterate over all field values
    fn values<'js>(&self, ctx: Ctx<'js>) -> JsResult<rquickjs::Value<'js>> {
        let array = rquickjs::Array::new(ctx.clone())?;
        for (i, (_, value)) in self.fields.iter().enumerate() {
            array.set(i, value.as_str())?;
        }
        Ok(array.into_value())
    }
}
