// core apis (imported as `core:events`)
pub mod events;
// pub mod streams;
// pub mod encoding;

// platform apis (imported as `lamprey:api`)
pub mod api;
pub mod env;
pub mod http;
pub mod log;
pub mod net;
pub mod register;
pub mod run; // rename to redex?
pub mod storage; // redo entirely?

// // future apis
// pub mod html;
// pub mod stream;
// pub mod wasm; // allow running wasm from quickjs?
// pub mod time; // maybe?
