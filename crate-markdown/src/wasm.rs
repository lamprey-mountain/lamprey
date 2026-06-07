// TODO: remove

use wasm_bindgen::prelude::*;

/// a markdown parser
#[wasm_bindgen(js_name = Parser)]
pub struct WasmParser {
    // TODO
}

/// parsed markdown
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[wasm_bindgen(js_name = Parsed)]
pub struct WasmParsed {
    // TODO
}

#[wasm_bindgen(js_class = Parser)]
impl WasmParser {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        todo!()
    }

    /// parse some markdown
    pub fn parse(&self, _markdown: &str) -> WasmParsed {
        todo!()
    }

    // pub fn strip_emoji(markdown: &str, allowed_emojis: JsValue) -> Result<JsValue, JsValue> {
    // pub fn render_markdown(markdown: &str) -> String {
}

#[wasm_bindgen(js_class = Parsed)]
impl WasmParsed {
    /// get the source string
    pub fn source(&self) -> String {
        todo!()
    }

    /// apply an edit
    pub fn edit(&mut self, _delete_start: u32, _delete_end: u32, _insert: &str) {
        todo!()
    }

    /// get the syntax tree
    pub fn tree(&self) -> JsValue {
        todo!()
    }

    /// render to html
    pub fn to_html(&self) -> String {
        todo!()
    }

    /// render to plaintext, stripping any formatting
    pub fn to_plain(&self) -> String {
        todo!()
    }
}
