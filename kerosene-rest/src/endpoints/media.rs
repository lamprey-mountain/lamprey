use crate::prelude::*;

// mod emoji;
// mod gifv;
pub mod media;
// mod stream;
// mod thumb;
// mod trickplay;

pub fn register(r: &mut crate::Routes) {
    media::register(r);
}
