use rquickjs::{
    Ctx, JsLifetime, Result as JsResult,
    class::{Trace, Tracer},
};

/// http request object
#[rquickjs::class]
#[derive(JsLifetime)]
pub struct TextEncoder {
    // TODO
}

// #[rquickjs::class]
// #[derive(JsLifetime)]
// pub struct TextEncoderStream {
//     // TODO
// }

// #[rquickjs::class]
// #[derive(JsLifetime)]
// pub struct TextDeccoder {
//     // TODO
// }

// #[rquickjs::class]
// #[derive(JsLifetime)]
// pub struct TextDeccoderStream {
//     // TODO
// }

impl<'js> Trace<'js> for TextEncoder {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl TextEncoder {
    #[qjs(constructor)]
    fn new() -> Self {
        todo!()
    }

    #[qjs(get)]
    fn method(&self, _ctx: Ctx<'_>) -> JsResult<String> {
        Ok("utf-8".to_string())
    }

    fn encode<'js>(&self, ctx: Ctx<'js>, string: rquickjs::String<'js>) -> JsResult<Vec<u8>> {
        todo!()
    }

    fn encode_into<'js>(
        &self,
        ctx: Ctx<'js>,
        string: rquickjs::String<'js>,
        target: rquickjs::Object<'js>, // NOTE: there's probably a better type to use here
    ) -> JsResult<Vec<u8>> {
        // Return value

        // An object, which contains two members:

        // read
        //     The number of UTF-16 code units from the source that have been converted to UTF-8. This may be less than string.length if uint8Array did not have enough space.
        // written
        //     The number of bytes modified in the destination Uint8Array. The bytes written are guaranteed to form complete UTF-8 byte sequences.

        // rquickjs_serde::to_value(ctx, value)
        todo!()
    }
}
