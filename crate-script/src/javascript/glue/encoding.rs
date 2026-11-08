use rquickjs::{
    ArrayBuffer, Coerced, Ctx, Exception, JsLifetime, Object, Result as JsResult, TypedArray,
    Value,
    class::{Trace, Tracer},
    prelude::Opt,
};

const STREAM_SOURCE: &str = include_str!("encoding.js");

/// encodes strings to bytes
#[rquickjs::class(rename = "TextEncoder")]
#[derive(Clone, Copy, JsLifetime)]
pub struct TextEncoder {}

/// decodes bytes into strings
#[rquickjs::class(rename = "TextDecoder")]
#[derive(Clone, JsLifetime)]
pub struct TextDecoder {
    encoding: String,
    fatal: bool,
    ignore_bom: bool,
    seen_bom: bool,
    pending: Vec<u8>,
}

#[derive(Default)]
struct TextDecoderOptions {
    fatal: bool,
    ignore_bom: bool,
}

impl<'js> Trace<'js> for TextEncoder {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

impl<'js> Trace<'js> for TextDecoder {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

/// install the text encoding globals
pub fn setup(ctx: &Ctx<'_>) -> JsResult<()> {
    rquickjs::Class::<TextEncoder>::define(&ctx.globals())?;
    rquickjs::Class::<TextDecoder>::define(&ctx.globals())?;
    ctx.eval::<(), _>(STREAM_SOURCE)
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl TextEncoder {
    #[qjs(constructor)]
    fn new() -> Self {
        Self {}
    }

    /// the encoding used by this encoder
    #[qjs(get)]
    fn encoding(&self) -> &str {
        "utf-8"
    }

    /// encode a string as utf-8
    fn encode<'js>(
        &self,
        ctx: Ctx<'js>,
        input: Opt<Coerced<String>>,
    ) -> JsResult<TypedArray<'js, u8>> {
        let input = input.0.map(|input| input.0).unwrap_or_default();
        TypedArray::new_copy(ctx, input.as_bytes())
    }

    /// encode as much of a string as fits into the target buffer
    fn encode_into<'js>(
        &self,
        ctx: Ctx<'js>,
        input: Coerced<String>,
        target: TypedArray<'js, u8>,
    ) -> JsResult<Object<'js>> {
        let input = input.0;
        let target = target
            .as_raw()
            .ok_or_else(|| Exception::throw_type(&ctx, "target buffer is detached"))?;

        // SAFETY: no javascript is executed while this slice is alive
        let target =
            unsafe { std::slice::from_raw_parts_mut(target.as_ptr() as *mut u8, target.len()) };

        let mut read = 0;
        let mut written = 0;

        for character in input.chars() {
            let mut buffer = [0u8; 4];
            let encoded = character.encode_utf8(&mut buffer).as_bytes();

            if written + encoded.len() > target.len() {
                break;
            }

            target[written..written + encoded.len()].copy_from_slice(encoded);
            written += encoded.len();
            read += character.len_utf16();
        }

        let result = Object::new(ctx)?;
        result.set("read", read)?;
        result.set("written", written)?;
        Ok(result)
    }
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl TextDecoder {
    #[qjs(constructor)]
    fn new<'js>(
        ctx: Ctx<'js>,
        label: Opt<Coerced<String>>,
        options: Opt<Value<'js>>,
    ) -> JsResult<Self> {
        let label = label.0.map(|label| label.0).unwrap_or_default();
        let label = if label.is_empty() {
            "utf-8".to_string()
        } else {
            label
        };

        let encoding = normalize_encoding(&label)
            .ok_or_else(|| Exception::throw_range(&ctx, "unsupported encoding"))?;
        let options = parse_decoder_options(&ctx, options)?;

        Ok(Self {
            encoding: encoding.to_string(),
            fatal: options.fatal,
            ignore_bom: options.ignore_bom,
            seen_bom: false,
            pending: Vec::new(),
        })
    }

    /// the canonical encoding name
    #[qjs(get)]
    fn encoding(&self) -> &str {
        &self.encoding
    }

    /// whether invalid data throws instead of being replaced
    #[qjs(get)]
    fn fatal(&self) -> bool {
        self.fatal
    }

    /// whether a leading byte order mark is preserved
    #[qjs(get, rename = "ignoreBOM")]
    fn ignore_bom(&self) -> bool {
        self.ignore_bom
    }

    /// decode bytes into a string
    fn decode<'js>(
        &mut self,
        ctx: Ctx<'js>,
        input: Opt<Value<'js>>,
        options: Opt<Value<'js>>,
    ) -> JsResult<String> {
        let mut bytes = match input.0 {
            Some(input) if !input.is_undefined() && !input.is_null() => {
                bytes_from_value(&ctx, input)?
            }
            _ => Vec::new(),
        };

        if !self.pending.is_empty() {
            let mut combined = std::mem::take(&mut self.pending);
            combined.extend_from_slice(&bytes);
            bytes = combined;
        }

        let stream = parse_stream_option(&ctx, options)?;

        if !self.ignore_bom && !self.seen_bom {
            let bom_len = bom_len(&self.encoding);

            let bom = bom(&self.encoding);
            if bytes.len() >= bom_len {
                if bytes.starts_with(bom) {
                    bytes.drain(..bom_len);
                }
                self.seen_bom = true;
            } else if stream && bytes == bom[..bytes.len()] {
                self.pending = bytes;
                return Ok(String::new());
            } else {
                self.seen_bom = true;
            }
        }

        let tail = if stream {
            incomplete_tail_len(&self.encoding, &bytes)
        } else {
            0
        };

        let ready = &bytes[..bytes.len() - tail];
        let decoded = decode_bytes(&self.encoding, ready, self.fatal)
            .ok_or_else(|| Exception::throw_type(&ctx, "invalid encoded data"))?;

        if stream {
            self.pending = bytes[bytes.len() - tail..].to_vec();
        } else {
            self.pending.clear();
        }

        Ok(decoded)
    }
}

fn parse_decoder_options<'js>(
    ctx: &Ctx<'js>,
    options: Opt<Value<'js>>,
) -> JsResult<TextDecoderOptions> {
    let Some(options) = options.0 else {
        return Ok(TextDecoderOptions::default());
    };

    if options.is_undefined() || options.is_null() {
        return Ok(TextDecoderOptions::default());
    }

    let options = options
        .into_object()
        .ok_or_else(|| Exception::throw_type(ctx, "options must be an object"))?;

    Ok(TextDecoderOptions {
        fatal: options.get::<_, Option<bool>>("fatal")?.unwrap_or(false),
        ignore_bom: options
            .get::<_, Option<bool>>("ignoreBOM")?
            .unwrap_or(false),
    })
}

fn parse_stream_option<'js>(ctx: &Ctx<'js>, options: Opt<Value<'js>>) -> JsResult<bool> {
    let Some(options) = options.0 else {
        return Ok(false);
    };

    if options.is_undefined() || options.is_null() {
        return Ok(false);
    }

    let options = options
        .into_object()
        .ok_or_else(|| Exception::throw_type(ctx, "options must be an object"))?;

    Ok(options.get::<_, Option<bool>>("stream")?.unwrap_or(false))
}

fn bytes_from_value<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Vec<u8>> {
    if let Some(buffer) = ArrayBuffer::from_value(value.clone()) {
        // SAFETY: copy the bytes immediately and don't call back into js while borrowed
        return unsafe {
            buffer
                .as_bytes()
                .map(|bytes| bytes.to_vec())
                .ok_or_else(|| Exception::throw_type(ctx, "buffer is detached"))
        };
    }

    if let Some(object) = value.as_object() {
        if let Some(buffer) = object.get::<_, Option<Value>>("buffer")? {
            if let Some(buffer) = ArrayBuffer::from_value(buffer) {
                let offset = object.get::<_, Option<usize>>("byteOffset")?.unwrap_or(0);
                let len = object
                    .get::<_, Option<usize>>("byteLength")?
                    .unwrap_or_else(|| buffer.len());

                // SAFETY: copy the bytes immediately and don't call back into js while borrowed
                if let Some(bytes) = unsafe { buffer.as_bytes() } {
                    let end = offset.saturating_add(len);
                    if end <= bytes.len() {
                        return Ok(bytes[offset..end].to_vec());
                    }
                }
            }
        }
    }

    Err(Exception::throw_type(
        ctx,
        "input must be an ArrayBuffer or ArrayBufferView",
    ))
}

fn normalize_encoding(label: &str) -> Option<&'static str> {
    let label = label.trim().to_ascii_lowercase();

    match label.as_str() {
        "utf-8" | "utf8" | "unicode-1-1-utf-8" => Some("utf-8"),
        "utf-16le" | "utf-16" | "unicode" => Some("utf-16le"),
        "utf-16be" => Some("utf-16be"),
        _ => None,
    }
}

fn bom(encoding: &str) -> &'static [u8] {
    match encoding {
        "utf-16le" => &[0xff, 0xfe],
        "utf-16be" => &[0xfe, 0xff],
        _ => &[0xef, 0xbb, 0xbf],
    }
}

fn bom_len(encoding: &str) -> usize {
    bom(encoding).len()
}

fn incomplete_tail_len(encoding: &str, bytes: &[u8]) -> usize {
    match encoding {
        "utf-16le" | "utf-16be" => {
            if bytes.len() % 2 != 0 {
                1
            } else if bytes.len() >= 2 {
                let last = if encoding == "utf-16le" {
                    u16::from_le_bytes([bytes[bytes.len() - 2], bytes[bytes.len() - 1]])
                } else {
                    u16::from_be_bytes([bytes[bytes.len() - 2], bytes[bytes.len() - 1]])
                };
                if (0xd800..=0xdbff).contains(&last) {
                    2
                } else {
                    0
                }
            } else {
                0
            }
        }
        _ => match std::str::from_utf8(bytes) {
            Ok(_) => 0,
            Err(error) if error.error_len().is_none() => bytes.len() - error.valid_up_to(),
            Err(_) => 0,
        },
    }
}

fn decode_bytes(encoding: &str, bytes: &[u8], fatal: bool) -> Option<String> {
    match encoding {
        "utf-16le" => decode_utf16(bytes, true, fatal),
        "utf-16be" => decode_utf16(bytes, false, fatal),
        _ => {
            if fatal {
                std::str::from_utf8(bytes).ok().map(str::to_string)
            } else {
                Some(String::from_utf8_lossy(bytes).into_owned())
            }
        }
    }
}

fn decode_utf16(bytes: &[u8], little_endian: bool, fatal: bool) -> Option<String> {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| {
            let bytes = [chunk[0], chunk[1]];
            if little_endian {
                u16::from_le_bytes(bytes)
            } else {
                u16::from_be_bytes(bytes)
            }
        })
        .collect();

    if fatal {
        String::from_utf16(&units).ok()
    } else {
        let mut decoded = String::from_utf16_lossy(&units);
        if bytes.len() % 2 != 0 {
            decoded.push('\u{fffd}');
        }
        Some(decoded)
    }
}

#[cfg(test)]
mod tests {
    use rquickjs::{Context, Promise, Runtime};

    use super::{TextDecoder, TextEncoder, setup};
    use crate::javascript::glue::streams;

    fn run(source: &str) -> String {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            streams::setup(&ctx).unwrap();
            setup(&ctx).unwrap();
            ctx.eval::<Promise, _>(source)
                .unwrap()
                .finish::<String>()
                .unwrap()
        })
    }

    #[test]
    fn encoder_roundtrip() {
        let result = run(r#"
                (async () => {
                    const encoder = new TextEncoder();
                    const bytes = encoder.encode("héllo");
                    return [
                        encoder.encoding,
                        ...bytes,
                        encoder.encodeInto("hello", new Uint8Array(3)).read,
                        encoder.encodeInto("hello", new Uint8Array(3)).written,
                    ].join(",");
                })()
            "#);

        assert_eq!(result, "utf-8,104,195,169,108,108,111,3,3");
    }

    #[test]
    fn decoder_stream() {
        let result = run(r#"
                (async () => {
                    const stream = new ReadableStream({
                        start(controller) {
                            controller.enqueue(new Uint8Array([104, 195]));
                            controller.enqueue(new Uint8Array([169, 108, 108, 111]));
                            controller.close();
                        },
                    });

                    let out = "";
                    for await (const chunk of stream.pipeThrough(new TextDecoderStream())) {
                        out += chunk;
                    }
                    return out;
                })()
            "#);

        assert_eq!(result, "héllo");
    }

    #[test]
    fn encoder_stream() {
        let result = run(r#"
                (async () => {
                    const decoder = new TextDecoder();
                    let out = "";
                    for await (const chunk of new ReadableStream({
                        start(controller) {
                            controller.enqueue("hello");
                            controller.enqueue(" ");
                            controller.enqueue("world");
                            controller.close();
                        },
                    }).pipeThrough(new TextEncoderStream())) {
                        out += decoder.decode(chunk);
                    }
                    return out;
                })()
            "#);

        assert_eq!(result, "hello world");
    }
}
