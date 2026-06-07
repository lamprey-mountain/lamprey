use crate::prelude::*;

// TODO: impl serde, use serde to wasm bindgen instead?
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct ParserConfig {
    /// the mrakdown format to parse
    pub format: ParserConfigFormat,
}

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum ParserConfigFormat {
    /// only parse inline markdown
    Inline,

    /// parse basic block level markdown
    Block,

    /// parse full markdown
    #[default]
    Full,
}
