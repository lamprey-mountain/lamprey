pub mod error;
pub mod logging;
pub mod plugin;
pub mod unfurler;
pub mod util;

// TODO: add tests

pub use plugin::{Plugin, PluginHtml, PluginHttp, PluginUrl};
pub use unfurler::Unfurler;

// builtin plugins
pub use plugin::direct_media::DirectMediaPlugin;
pub use plugin::json_ld::JsonLdPlugin;
pub use plugin::microdata::MicrodataPlugin;
pub use plugin::opengraph::OpengraphPlugin;
pub use plugin::wikipedia::WikipediaPlugin;
