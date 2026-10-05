use crate::prelude::*;

pub mod emoji;
pub mod gifv;
pub mod media;
// pub mod stream;
pub mod thumb;
// pub mod trickplay;

mod util;

pub fn register(r: &mut crate::Routes) {
    emoji::register(r);
    gifv::register(r);
    media::register(r);
    thumb::register(r);
}
