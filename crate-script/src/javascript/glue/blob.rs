use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use rquickjs::{
    ArrayBuffer, Class, Coerced, Ctx, Exception, JsLifetime, Promise, Result as JsResult,
    TypedArray, Value,
    class::{Trace, Tracer},
    prelude::Opt,
};

/// immutable raw binary data
#[rquickjs::class(rename = "Blob")]
#[derive(Clone, JsLifetime)]
pub struct Blob {
    pub(crate) data: Bytes,
    pub(crate) ty: String,
}

/// a blob with an associated name and modification time
#[rquickjs::class(rename = "File")]
#[derive(Clone, JsLifetime)]
pub struct File {
    pub(crate) blob: Blob,
    pub(crate) name: String,
    pub(crate) last_modified: i64,
}

impl<'js> Trace<'js> for Blob {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

impl<'js> Trace<'js> for File {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

impl Blob {
    pub(crate) fn from_bytes(data: Bytes, ty: impl Into<String>) -> Self {
        Self {
            data,
            ty: normalize_type(&ty.into()),
        }
    }

    fn from_parts<'js>(ctx: &Ctx<'js>, parts: Opt<Value<'js>>, ty: &str) -> JsResult<Self> {
        let mut data = Vec::new();

        if let Some(parts) = parts.0 {
            if !parts.is_undefined() && !parts.is_null() {
                let parts = parts
                    .into_array()
                    .ok_or_else(|| Exception::throw_type(ctx, "Blob parts must be an array"))?;

                for part in parts.iter::<Value>() {
                    append_part(ctx, &mut data, part?)?;
                }
            }
        }

        Ok(Self {
            data: Bytes::from(data),
            ty: normalize_type(ty),
        })
    }

    fn options_type<'js>(options: Opt<Value<'js>>) -> String {
        options
            .0
            .and_then(|options| options.into_object())
            .and_then(|options| {
                options
                    .get::<_, Option<Coerced<String>>>("type")
                    .ok()
                    .flatten()
                    .map(|ty| ty.0)
            })
            .unwrap_or_default()
    }
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl Blob {
    #[qjs(constructor)]
    fn new<'js>(ctx: Ctx<'js>, parts: Opt<Value<'js>>, options: Opt<Value<'js>>) -> JsResult<Self> {
        let ty = Self::options_type(options);
        Self::from_parts(&ctx, parts, &ty)
    }

    /// size of the blob in bytes
    #[qjs(get)]
    fn size(&self) -> usize {
        self.data.len()
    }

    /// mime type of the blob
    #[qjs(get, rename = "type")]
    fn ty(&self) -> &str {
        &self.ty
    }

    /// returns the blob contents as text
    fn text<'js>(&self, ctx: Ctx<'js>) -> JsResult<Promise<'js>> {
        let (promise, resolve, _reject) = ctx.promise()?;
        let text = String::from_utf8_lossy(&self.data).to_string();
        resolve.call::<_, ()>((text,))?;
        Ok(promise)
    }

    /// returns the blob contents as an ArrayBuffer
    fn array_buffer<'js>(&self, ctx: Ctx<'js>) -> JsResult<Promise<'js>> {
        let (promise, resolve, _reject) = ctx.promise()?;
        let buffer = ArrayBuffer::new_copy(ctx, self.data.as_ref())?;
        resolve.call::<_, ()>((buffer,))?;
        Ok(promise)
    }

    /// returns the blob contents as a Uint8Array
    fn bytes<'js>(&self, ctx: Ctx<'js>) -> JsResult<Promise<'js>> {
        let (promise, resolve, _reject) = ctx.promise()?;
        let bytes = TypedArray::<u8>::new_copy(ctx, self.data.as_ref())?;
        resolve.call::<_, ()>((bytes,))?;
        Ok(promise)
    }

    /// returns a new blob containing the specified byte range
    fn slice(&self, start: Opt<f64>, end: Opt<f64>, content_type: Opt<Coerced<String>>) -> Self {
        let size = self.data.len();
        let start = normalize_index(start.0.unwrap_or(0.0), size);
        let end = normalize_index(end.0.unwrap_or(size as f64), size);

        let data = if end > start {
            self.data.slice(start..end)
        } else {
            Bytes::new()
        };

        let ty = content_type
            .0
            .map(|ty| ty.0)
            .map(|ty| normalize_type(&ty))
            .unwrap_or_default();

        Self { data, ty }
    }

    /// returns a ReadableStream of the blob contents
    fn stream<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        super::streams::readable_from_bytes(&ctx, &self.data)
    }
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl File {
    #[qjs(constructor)]
    fn new<'js>(
        ctx: Ctx<'js>,
        file_bits: Opt<Value<'js>>,
        name: Coerced<String>,
        options: Opt<Value<'js>>,
    ) -> JsResult<Self> {
        let (ty, last_modified) = parse_file_options(&ctx, options)?;
        let blob = Blob::from_parts(&ctx, file_bits, &ty)?;

        Ok(Self {
            blob,
            name: name.0,
            last_modified,
        })
    }

    /// name of the file
    #[qjs(get)]
    fn name(&self) -> &str {
        &self.name
    }

    /// last modification time in milliseconds since the unix epoch
    #[qjs(get, rename = "lastModified")]
    fn last_modified(&self) -> i64 {
        self.last_modified
    }

    /// size of the file in bytes
    #[qjs(get)]
    fn size(&self) -> usize {
        self.blob.data.len()
    }

    /// mime type of the file
    #[qjs(get, rename = "type")]
    fn ty(&self) -> &str {
        &self.blob.ty
    }

    /// returns the file contents as text
    fn text<'js>(&self, ctx: Ctx<'js>) -> JsResult<Promise<'js>> {
        self.blob.text(ctx)
    }

    /// returns the file contents as an ArrayBuffer
    fn array_buffer<'js>(&self, ctx: Ctx<'js>) -> JsResult<Promise<'js>> {
        self.blob.array_buffer(ctx)
    }

    /// returns the file contents as a Uint8Array
    fn bytes<'js>(&self, ctx: Ctx<'js>) -> JsResult<Promise<'js>> {
        self.blob.bytes(ctx)
    }

    /// returns a new blob containing the specified byte range
    fn slice(&self, start: Opt<f64>, end: Opt<f64>, content_type: Opt<Coerced<String>>) -> Blob {
        self.blob.slice(start, end, content_type)
    }

    /// returns a ReadableStream of the file contents
    fn stream<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        self.blob.stream(ctx)
    }
}

fn parse_file_options<'js>(ctx: &Ctx<'js>, options: Opt<Value<'js>>) -> JsResult<(String, i64)> {
    let Some(options) = options.0 else {
        return Ok((String::new(), now_millis()));
    };

    if options.is_undefined() || options.is_null() {
        return Ok((String::new(), now_millis()));
    }

    let options = options
        .into_object()
        .ok_or_else(|| Exception::throw_type(ctx, "File options must be an object"))?;

    let ty = options
        .get::<_, Option<Coerced<String>>>("type")?
        .map(|ty| ty.0)
        .unwrap_or_default();

    let last_modified = options
        .get::<_, Option<f64>>("lastModified")?
        .map(|value| value as i64)
        .unwrap_or_else(now_millis);

    Ok((ty, last_modified))
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn append_part<'js>(ctx: &Ctx<'js>, data: &mut Vec<u8>, part: Value<'js>) -> JsResult<()> {
    if let Some(string) = part.as_string() {
        data.extend_from_slice(string.to_string()?.as_bytes());
        return Ok(());
    }

    if let Ok(blob) = Class::<Blob>::from_value(&part) {
        data.extend_from_slice(&blob.borrow().data);
        return Ok(());
    }

    if let Some(buffer) = ArrayBuffer::from_value(part.clone()) {
        // SAFETY: copy the bytes immediately and don't call back into js while borrowed
        if let Some(bytes) = unsafe { buffer.as_bytes() } {
            data.extend_from_slice(bytes);
            return Ok(());
        }
    }

    // typed arrays and DataViews
    if let Some(obj) = part.as_object() {
        if let Some(buffer) = obj.get::<_, Option<Value>>("buffer")? {
            if let Some(buffer) = ArrayBuffer::from_value(buffer) {
                let offset = obj.get::<_, Option<usize>>("byteOffset")?.unwrap_or(0);
                let len = obj
                    .get::<_, Option<usize>>("byteLength")?
                    .unwrap_or_else(|| buffer.len());

                // SAFETY: copy the bytes immediately and don't call back into js while borrowed
                if let Some(bytes) = unsafe { buffer.as_bytes() } {
                    let end = offset.saturating_add(len);
                    if end <= bytes.len() {
                        data.extend_from_slice(&bytes[offset..end]);
                        return Ok(());
                    }
                }
            }
        }
    }

    Err(Exception::throw_type(
        ctx,
        "Blob part must be a string, Blob, ArrayBuffer, or ArrayBufferView",
    ))
}

fn normalize_type(ty: &str) -> String {
    if ty.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        ty.to_ascii_lowercase()
    } else {
        String::new()
    }
}

fn normalize_index(value: f64, size: usize) -> usize {
    if value.is_nan() {
        return 0;
    }

    let size = size as f64;
    if value < 0.0 {
        (size + value).clamp(0.0, size) as usize
    } else {
        value.clamp(0.0, size) as usize
    }
}

#[cfg(test)]
mod tests {
    use rquickjs::{Class, Context, Runtime};

    use super::{Blob, File};

    #[test]
    fn construct_and_slice() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            Class::<Blob>::define(&ctx.globals()).unwrap();

            let blob: Class<Blob> = ctx
                .eval(
                    r#"
                        new Blob(
                            ["hello ", new Uint8Array([119, 111, 114, 108, 100]), "!"],
                            { type: "TEXT/PLAIN" }
                        ).slice(1, -1, "text/HTML")
                    "#,
                )
                .unwrap();

            assert_eq!(blob.borrow().data.as_ref(), b"ello world");
            assert_eq!(blob.borrow().ty, "text/html");
        });
    }

    #[test]
    fn construct_from_dataview() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            Class::<Blob>::define(&ctx.globals()).unwrap();

            let blob: Class<Blob> = ctx
                .eval(
                    r#"
                        new Blob([
                            new DataView(new Uint8Array([1, 2, 3, 4]).buffer, 1, 2)
                        ])
                    "#,
                )
                .unwrap();

            assert_eq!(blob.borrow().data.as_ref(), &[2, 3]);
            assert!(blob.borrow().ty.is_empty());
        });
    }

    #[test]
    fn file_extends_blob() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            Class::<Blob>::define(&ctx.globals()).unwrap();
            Class::<File>::define(&ctx.globals()).unwrap();

            let blob = Class::<Blob>::prototype(&ctx).unwrap().unwrap();
            let file = Class::<File>::prototype(&ctx).unwrap().unwrap();
            file.set_prototype(Some(&blob)).unwrap();

            let file: Class<File> = ctx
                .eval(
                    r#"
                        new File(["hello"], "greeting.txt", {
                            type: "TEXT/PLAIN",
                            lastModified: 1234,
                        })
                    "#,
                )
                .unwrap();

            assert_eq!(file.borrow().blob.data.as_ref(), b"hello");
            assert_eq!(file.borrow().blob.ty, "text/plain");
            assert_eq!(file.borrow().name, "greeting.txt");
            assert_eq!(file.borrow().last_modified, 1234);

            let is_blob: bool = ctx.eval("new File([], 'a.txt') instanceof Blob").unwrap();
            assert!(is_blob);

            let sliced: Class<Blob> = ctx
                .eval("new File(['hello'], 'a.txt').slice(1, 3)")
                .unwrap();
            assert_eq!(sliced.borrow().data.as_ref(), b"el");
        });
    }
}
