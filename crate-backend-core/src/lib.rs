pub mod error;
pub mod state;

pub use common;
pub use sqlx;
pub use uuid;

pub mod prelude {
    use std::sync::Arc;

    pub use crate::error::{Error, Result};
    pub use crate::state::{ServerState, ServerStateInner};

    pub use common;
    pub use sqlx;
    pub use uuid;

    pub type SState = Arc<ServerState>;
}
