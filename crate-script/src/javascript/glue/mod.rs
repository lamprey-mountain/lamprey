// web standard apis (available globally)
// pub mod abort;
// pub mod compression;
// pub mod encoding;
pub mod http;
// pub mod url;
// pub mod streams;

// TODO: maybe impl more standard apis?
// - performance
// - crypto
// - Blob, File
// - dom interfaces like Event, EventTarget
// - webassembly
// - web workers? this can possibly be done via redexes
// - also see https://min-common-api.proposal.wintertc.org/
// - i should probably have console, but handling and formatting arbitrary js data seems a bit too difficult for now

// platform apis (imported as `lamprey:api`)
// pub mod api;
// pub mod env;
pub mod events; // maybe load Emitter into globals automatically?
// pub mod html;
pub mod log;
// pub mod net;
pub mod register;
// pub mod run; // rename to redex?
// pub mod storage; // redo entirely?
// pub mod stream; // may be confused with web standard streams
