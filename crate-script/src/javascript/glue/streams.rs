use rquickjs::{Array, Ctx, Function, Object, Result as JsResult, TypedArray, Value};

const SOURCE: &str = include_str!("streams.js");
const CHUNK_SIZE: usize = 64 * 1024;

/// install the web streams globals
pub fn setup(ctx: &Ctx<'_>) -> JsResult<()> {
    ctx.eval::<(), _>(SOURCE)
}

/// create a readable stream from raw bytes
pub(crate) fn readable_from_bytes<'js>(ctx: &Ctx<'js>, data: &[u8]) -> JsResult<Value<'js>> {
    let globals = ctx.globals();
    let readable: Object<'js> = globals.get("ReadableStream")?;
    let from: Function<'js> = readable.get("from")?;

    let chunks = Array::new(ctx.clone())?;
    for (index, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
        let chunk = TypedArray::<u8>::new_copy(ctx.clone(), chunk)?;
        chunks.set(index, chunk)?;
    }

    from.call((chunks,))
}

#[cfg(test)]
mod tests {
    use rquickjs::{Class, Context, Promise, Runtime};

    use super::setup;
    use crate::javascript::glue::blob::Blob;

    fn run(source: &str) -> String {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            Class::<Blob>::define(&ctx.globals()).unwrap();
            setup(&ctx).unwrap();
            ctx.eval::<Promise, _>(source)
                .unwrap()
                .finish::<String>()
                .unwrap()
        })
    }

    #[test]
    fn readable_stream() {
        let result = run(r#"
                (async () => {
                    const stream = new ReadableStream({
                        start(controller) {
                            controller.enqueue("a");
                            controller.enqueue("b");
                            controller.close();
                        },
                    });

                    const reader = stream.getReader();
                    let out = "";
                    for (;;) {
                        const result = await reader.read();
                        if (result.done) break;
                        out += result.value;
                    }
                    return out;
                })()
            "#);

        assert_eq!(result, "ab");
    }

    #[test]
    fn writable_stream() {
        let result = run(r#"
                (async () => {
                    let out = "";
                    const stream = new WritableStream({
                        write(chunk) {
                            out += chunk;
                        },
                    });

                    const writer = stream.getWriter();
                    await writer.write("a");
                    await writer.write("b");
                    await writer.close();
                    return out;
                })()
            "#);

        assert_eq!(result, "ab");
    }

    #[test]
    fn transform_stream() {
        let result = run(r#"
                (async () => {
                    const stream = new TransformStream({
                        transform(chunk, controller) {
                            controller.enqueue(chunk.toUpperCase());
                        },
                    });

                    const writer = stream.writable.getWriter();
                    await writer.write("a");
                    await writer.write("b");
                    await writer.close();

                    const reader = stream.readable.getReader();
                    let out = "";
                    for (;;) {
                        const result = await reader.read();
                        if (result.done) break;
                        out += result.value;
                    }
                    return out;
                })()
            "#);

        assert_eq!(result, "AB");
    }

    #[test]
    fn pipe_through() {
        let result = run(r#"
                (async () => {
                    const source = new ReadableStream({
                        start(controller) {
                            controller.enqueue("a");
                            controller.enqueue("b");
                            controller.close();
                        },
                    });

                    const transform = new TransformStream({
                        transform(chunk, controller) {
                            controller.enqueue(chunk.toUpperCase());
                        },
                    });

                    const reader = source.pipeThrough(transform).getReader();
                    let out = "";
                    for (;;) {
                        const result = await reader.read();
                        if (result.done) break;
                        out += result.value;
                    }
                    return out;
                })()
            "#);

        assert_eq!(result, "AB");
    }

    #[test]
    fn tee_and_async_iteration() {
        let result = run(r#"
                (async () => {
                    const stream = new ReadableStream({
                        start(controller) {
                            controller.enqueue("a");
                            controller.enqueue("b");
                            controller.close();
                        },
                    });

                    const [left, right] = stream.tee();
                    let out = "";
                    for await (const chunk of left) out += chunk;
                    for await (const chunk of right) out += chunk;
                    return out;
                })()
            "#);

        assert_eq!(result, "abab");
    }

    #[test]
    fn byob_enqueue() {
        let result = run(r#"
                (async () => {
                    const stream = new ReadableStream({
                        type: "bytes",
                        start(controller) {
                            controller.enqueue(new Uint8Array([1, 2, 3]));
                            controller.close();
                        },
                    });

                    const reader = stream.getReader({ mode: "byob" });
                    const first = await reader.read(new Uint8Array(5));
                    const second = await reader.read(new Uint8Array(5));
                    return [
                        first.done,
                        ...first.value,
                        second.done,
                        second.value.byteLength,
                    ].join(",");
                })()
            "#);

        assert_eq!(result, "false,1,2,3,true,0");
    }

    #[test]
    fn byte_stream_default_reader() {
        let result = run(r#"
                (async () => {
                    const stream = new ReadableStream({
                        type: "bytes",
                        start(controller) {
                            controller.enqueue(new Uint8Array([1, 2]));
                            controller.close();
                        },
                    });

                    const reader = stream.getReader();
                    const first = await reader.read();
                    const second = await reader.read();
                    return [first.done, ...first.value, second.done].join(",");
                })()
            "#);

        assert_eq!(result, "false,1,2,true");
    }

    #[test]
    fn byob_respond() {
        let result = run(r#"
                (async () => {
                    let sent = false;
                    const stream = new ReadableStream({
                        type: "bytes",
                        pull(controller) {
                            if (sent) {
                                controller.close();
                                return;
                            }
                            sent = true;
                            controller.byobRequest.view[0] = 42;
                            controller.byobRequest.view[1] = 43;
                            controller.byobRequest.respond(2);
                        },
                    });

                    const reader = stream.getReader({ mode: "byob" });
                    const result = await reader.read(new Uint8Array(4));
                    return [result.done, ...result.value].join(",");
                })()
            "#);

        assert_eq!(result, "false,42,43");
    }

    #[test]
    fn compatibility_globals() {
        let result = run(r#"
                (async () => {
                    const names = [
                        "ByteLengthQueuingStrategy",
                        "CountQueuingStrategy",
                        "ReadableByteStreamController",
                        "ReadableStream",
                        "ReadableStreamBYOBReader",
                        "ReadableStreamBYOBRequest",
                        "ReadableStreamDefaultController",
                        "ReadableStreamDefaultReader",
                        "TransformStream",
                        "TransformStreamDefaultController",
                        "WritableStream",
                        "WritableStreamDefaultController",
                        "WritableStreamDefaultWriter",
                    ];
                    return names.filter((name) => typeof globalThis[name] !== "function").join(",");
                })()
            "#);

        assert!(result.is_empty(), "missing globals: {result}");
    }

    #[test]
    fn blob_stream() {
        let result = run(r#"
                (async () => {
                    const blob = new Blob(["hello", " ", "world"]);
                    const reader = blob.stream().getReader();
                    let out = "";
                    for (;;) {
                        const result = await reader.read();
                        if (result.done) break;
                        out += String.fromCharCode(...result.value);
                    }
                    return out;
                })()
            "#);

        assert_eq!(result, "hello world");
    }
}
