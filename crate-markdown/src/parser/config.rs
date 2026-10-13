// TODO: impl serde, use serde to wasm bindgen instead?
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct ParserConfig {
    /// the markdown format to parse
    pub format: ParserConfigFormat,
    // /// valid url schemes
    // pub url_schemes: Vec<String>,
    // TODO: valid url schemes (default to http, https, mailto, tel, magnet. maybe ssh, telnet, ftp, ftps.)
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
