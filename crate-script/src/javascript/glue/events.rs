use rquickjs::{embed, loader::Bundle};

pub static BUNDLE: Bundle = embed! {
    "core:events": "src/javascript/glue/events.js",
};
