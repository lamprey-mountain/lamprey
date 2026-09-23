use crate::prelude::*;

// mod emoji;
// mod gifv;
pub mod media;
pub mod thumb;
// mod stream;
// mod trickplay;

mod util;

pub fn register(r: &mut crate::Routes) {
    media::register(r);
    thumb::register(r);
}
