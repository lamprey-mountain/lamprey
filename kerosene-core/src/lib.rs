// TEMP: proxying for now?
// TODO: write out this crate
// pub use lamprey_backend_core::queue;

pub mod compat;
pub mod config;
pub mod database;
pub mod error;
pub mod ffmpeg;
pub mod observability;
pub mod types;

// pure logic/state for various resources, no io
// TODO: implement?
#[cfg(any())]
pub mod actors {
    pub struct RoomData {
        // copy from kerosene-services/src/services/rooms/types.rs?
    }

    impl RoomData {
        pub fn handle_sync(&mut self, sync: lamprey::v1::types::MessageSync) {
            // update state here
            todo!()
        }
    }

    enum RoomCommand {}
    enum RoomEvent {}

    // logic for channels, maybe users?
}

/// common types used everywhere in kerosene
pub mod prelude {
    pub use crate::error::{
        ApiError, ApiResult, CoreResult, ErrorCode, Result, ServerError,
        ServerResult,
    };
    pub use bytes::Bytes;

    // TODO: use more types in prelude?
    // pub use lamprey::v1::types::{UserId, RoomId, MediaId};
}

// NOTE: instead of a generic util module, maybe i should create a dedicated module for each feature
// mod util;
