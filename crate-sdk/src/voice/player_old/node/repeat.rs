use crate::voice::player::node::Node;

pub struct Repeat<N: Node> {
    source: N,
    // TODO
}

#[derive(Clone)]
pub struct RepeatHandle {
    // TODO
}

// impl RepeatHandle {
//     pub fn repeat_kind(&self) -> RepeatKind;
//     pub fn set_repeat_kind(&self, kind: RepeatKind);

//     pub fn disable(&self);
//     pub fn track(&self);
//     pub fn queue(&self); // only expose for queues?
// }

// only allow setting RepeatKind::Queue for repeats that that wrap queues
// impl RepeatHandle<Queue> {
//     pub fn queue(&self);
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatKind {
    Disabled,
    Track,
    Queue,
}
