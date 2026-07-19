use crate::voice::player::util::{MediaKind, MediaSource};

/// a [`MediaSource`] that can be interacted with externally
// TODO: rename to MediaNode
pub trait Node: MediaSource<Self::Media> + Sized + Send + 'static {
    type Handle: Handle;
    type Media: MediaKind;

    /// get a handle that can be used to access this node
    fn handle(&self) -> Self::Handle;

    fn add_pauser(self) -> Pause<Self> {
        Pause::new(self)
    }

    fn add_volume(self) -> Volume<Self> {
        Volume::new(self)
    }
}

// pub enum AnyNode<H> {
//     Audio(Box<dyn Node<Media = Audio, Handle = H>>),
//     Video(Box<dyn Node<Media = Video, Handle = H>>),
// }

/// a handle to a node
// TODO: rename to MediaHandle
// TODO: maybe remove entirely?
pub trait Handle: Clone {}

mod pause;
// mod queue;
// mod repeat;
mod source;
mod volume;

pub use pause::*;
// pub use queue::*;
// pub use repeat::*;
pub use source::*;
pub use volume::*;
