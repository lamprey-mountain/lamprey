use uuid::Uuid;

use crate::{
    ast::{Ast, Render},
    tree::node::Node,
};

/// a transformation to replace custom emoji with `:name:` unless they're allowed
// TODO
pub struct StripEmoji {
    /// what emoji are allowed
    pub allowed: Vec<Uuid>,
    // ast: &'a dyn Ast,
}

impl StripEmoji {
    // pub fn new(ast: impl Into<Box<dyn Ast>>) -> Self {
    //     todo!()
    // }
}

// impl Render for StripEmoji {
//     // TODO
// }

// TODO: utility to toggle a task list item
