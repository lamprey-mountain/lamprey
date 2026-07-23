// TEMP: proxying for now?
// TODO: write out this crate
pub use lamprey_backend_core::config;
// pub use lamprey_backend_core::queue;
pub use lamprey_backend_core::types;

// TODO: implement(?)
pub mod database;
pub mod error;

// pure logic/state for various resources, no io
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

    // logic for channels, maybe users?
}

// TEMP: compatibility types for migration
pub mod compat;

/// common types used everywhere in backend
pub mod prelude {
    pub use crate::error::{ApiError, ApiResult, ServerError, ServerResult};

    // TODO: use more types in prelude?
    // pub use lamprey::v1::types::{UserId, RoomId, MediaId};
}
