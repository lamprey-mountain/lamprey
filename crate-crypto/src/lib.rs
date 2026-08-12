mod error;
mod group;
mod identity;
mod keysharing;
mod manager;
mod media;
mod serialize;
mod util;
mod voice;

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(test)]
mod tests;

// for internal use
pub(crate) mod prelude {
    #[cfg(feature = "wasm")]
    pub use wasm_bindgen::prelude::*;

    pub use crate::error::Error;
    pub type Result<T> = core::result::Result<T, Error>;

    #[cfg(not(feature = "sync"))]
    pub type Ref<T> = ::std::rc::Rc<T>;

    #[cfg(feature = "sync")]
    pub type Ref<T> = ::std::sync::Arc<T>;
}

pub use group::EncryptionChannel;
pub use manager::{Action, Actions, Encryption};
